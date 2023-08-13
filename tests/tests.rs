use auto_sitemap::*;
use chrono::{DateTime, Utc};
use url::Url;

#[test]
fn test_serialize_and_deserialize() {
    let str_representation = include_str!("data/simple-sitemap.xml")
        .trim()
        .replace("\r\n", "\n");

    let sitemap = Sitemap {
        pages: vec![Page {
            url: Url::parse("https://example.com").unwrap(),
            lastmod: Some(DateTime::<Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp_opt(61, 0).unwrap(),
                Utc,
            )),
            md5_hash: Some("0123456789abcdef0123456789abcdef".into()),
        }],
    };

    let mut buf = std::io::BufWriter::new(Vec::new());
    sitemap.serialize(&mut buf).unwrap();
    let serialized = String::from_utf8(buf.into_inner().unwrap()).unwrap();
    let deserialized = Sitemap::deserialize(serialized.as_bytes()).unwrap();

    pretty_assertions::assert_eq!(serialized, str_representation);
    pretty_assertions::assert_eq!(deserialized, sitemap);
}

mod sitemap {
    use super::*;
    use axum::response::Html;
    use axum::{routing::get, Router};
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_generation_and_update() {
        let start_time = chrono::Utc::now();

        let mut new_sitemap = generate_sitemap().await.unwrap();

        let correct_urls = vec![
            Url::parse("http://localhost:3000/").unwrap(),
            Url::parse("http://localhost:3000/a").unwrap(),
            Url::parse("http://localhost:3000/b").unwrap(),
            Url::parse("http://localhost:3000/c").unwrap(),
            // Shouldn't be reachable by crawling:
            // Url::parse("http://localhost:3000/d").unwrap(),
        ];

        for (page, correct_url) in new_sitemap.pages.iter().zip(correct_urls.iter()) {
            pretty_assertions::assert_eq!(page.url, correct_url.clone());
        }

        let old_sitemap_str = include_str!("data/old-sitemap.xml");

        let old_sitemap = Sitemap::deserialize(old_sitemap_str.as_bytes()).unwrap();

        new_sitemap.combine_with_old_sitemap(&old_sitemap).unwrap();

        let end_time = chrono::Utc::now();

        let updated_urls = vec![
            Url::parse("http://localhost:3000/a").unwrap(),
            Url::parse("http://localhost:3000/b").unwrap(),
            Url::parse("http://localhost:3000/c").unwrap(),
        ];
        for (page, correct_url) in new_sitemap.pages.iter().zip(correct_urls.iter()) {
            pretty_assertions::assert_eq!(page.url, correct_url.clone());
            let lastmod = page.lastmod.unwrap();

            if updated_urls.contains(&page.url) {
                more_asserts::assert_lt!(start_time, lastmod);
                more_asserts::assert_lt!(lastmod, end_time);
            } else {
                more_asserts::assert_lt!(lastmod, start_time);
            }
        }
    }

    async fn generate_sitemap() -> Result<Sitemap, String> {
        let app = Router::new()
            .route("/", get(root))
            .route("/a", get(a))
            .route("/b", get(b))
            .route("/c", get(c))
            .route("/d", get(d));

        let port = 3000;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let server = axum::Server::bind(&addr).serve(app.into_make_service());

        // Prepare some signal for when the server should start shutting down...
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let graceful = server.with_graceful_shutdown(async {
            rx.await.ok();
        });

        println!("Listening on http://localhost:{port}");
        tokio::spawn(async {
            if let Err(e) = graceful.await {
                eprintln!("server error: {}", e);
            }
        });

        let mut sitemap = Sitemap::try_from_url_str("http://localhost:3000").await?;
        sitemap.sort_by_url();

        // Shut down the server...
        let _ = tx.send(());

        Ok(sitemap)
    }

    // Reachable from / and /c.
    // Also ensure it works on all OSs by importing a file, which may have \n or \r\n line endings.
    async fn root() -> Html<&'static str> {
        Html(include_str!("data/root.html"))
    }

    // Reachable from /.
    async fn a() -> Html<&'static str> {
        Html(
            r#"
<html>
  <body>
    <a href="/b">Reachable from home and a</a>
    <a href="/c">Reachable from home a</a>
  </body>
</html>
"#,
        )
    }

    // Reachable from /home and /a.
    async fn b() -> Html<&'static str> {
        Html(
            r#"
<html>
  <body>
  </body>
</html>
"#,
        )
    }

    // Reachable from /a and /c.
    async fn c() -> Html<&'static str> {
        Html(
            r#"
<html>
  <body>
    <a href="/c">Reachable from itself</a>
    <a href="/root">Reachable from home and c</a>
  </body>
</html>
"#,
        )
    }

    // Unreachable.
    async fn d() -> Html<&'static str> {
        Html(
            r#"
            <html><body>
                <h1>Unreachable!</h1>
            </body></html>
        "#,
        )
    }
}

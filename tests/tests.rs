use auto_sitemap::*;
use chrono::{DateTime, Utc};
use url::Url;

#[test]
fn test_serialize_and_deserialize() {
    let str_representation = r#"<?xml version="1.0" encoding="utf-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>1970-01-01T00:01:01Z</lastmod>
    <xhtml:meta name="auto_sitemap_md5_hash" content="0123456789abcdef0123456789abcdef" />
  </url>
</urlset>"#;

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

        let old_sitemap_str = r#"<?xml version="1.0" encoding="utf-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
    <url>
        <loc>http://localhost:3000/</loc>
        <lastmod>2020-01-05T00:00:00Z</lastmod>
        <xhtml:meta name="auto_sitemap_md5_hash" content="5c3e45e7c1558d67050cb19c0dc390fd" />
        }
    </url>
    <url>
        <loc>http://localhost:3000/a</loc>
        <lastmod>2020-01-06T00:00:00Z</lastmod>
    </url>
    <url>
        <loc>http://localhost:3000/b</loc>
        <lastmod>2020-01-07T00:00:00Z</lastmod>
        <xhtml:meta name="auto_sitemap_md5_hash" content="0123456789abcdef0123456789abcdef" />
    </url>
    <url>
        <loc>http://localhost:3000/old-nonexistent-page-with-hash</loc>
        <lastmod>2020-01-08T00:00:00Z</lastmod>
        <xhtml:meta name="auto_sitemap_md5_hash" content="0123456789abcdef0123456789abcdef" />
    </url>
    <url>
        <loc>http://localhost:3000/old-nonexistent-page-without-hash</loc>
        <lastmod>2020-01-09T00:00:00Z</lastmod>
    </url>
</urlset>"#;

        let old_sitemap = Sitemap::deserialize(old_sitemap_str.as_bytes()).unwrap();

        new_sitemap.combine_with_old_sitemap(&old_sitemap).unwrap();

        let updated_urls = vec![
            Url::parse("http://localhost:3000/a").unwrap(),
            Url::parse("http://localhost:3000/b").unwrap(),
            Url::parse("http://localhost:3000/c").unwrap(),
        ];
        for (page, correct_url) in new_sitemap.pages.iter().zip(correct_urls.iter()) {
            pretty_assertions::assert_eq!(page.url, correct_url.clone());
            let lastmod = page.lastmod.unwrap();

            // Lastmod should be updated to less than 1 second after `start_time`.
            if updated_urls.contains(&page.url) {
                assert!(lastmod > start_time);
                assert!(lastmod < start_time + chrono::Duration::seconds(1));
            } else {
                assert!(lastmod < start_time);
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

    async fn root() -> Html<&'static str> {
        Html(
            r#"
            <html><body>
                <a href="/a">Reachable from home</a>
                <a href="/b">Reachable from home and a</a>
            </body></html>
        "#,
        )
    }

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

    async fn b() -> Html<&'static str> {
        Html(
            r#"
            <html></html>
        "#,
        )
    }

    async fn c() -> Html<&'static str> {
        Html(
            r#"
            <html></html>
        "#,
        )
    }

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

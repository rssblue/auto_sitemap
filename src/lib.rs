use chrono::{DateTime as ChronoDateTime, Utc};
use spider::website::Website;
use url::Url as UrlUrl;
use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, PartialEq)]
struct DateTime<Tz: chrono::TimeZone>(ChronoDateTime<Tz>);

#[derive(Debug, PartialEq)]
struct Url(UrlUrl);

/// Sitemap of the website.
#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
#[yaserde(
    rename = "urlset",
    namespace = "http://www.sitemaps.org/schemas/sitemap/0.9"
    namespace = "xhtml: http://www.w3.org/1999/xhtml"
)]
struct Sitemap {
    #[yaserde(rename = "url")]
    pages: Vec<Page>,
}

#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
struct Page {
    loc: Option<Url>,
    lastmod: Option<DateTime<Utc>>,
    #[yaserde(prefix = "xhtml")]
    meta: Option<Meta>,
}

#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
#[yaserde(namespace = "xhtml: http://www.w3.org/1999/xhtml")]
struct Meta {
    #[yaserde(attribute)]
    name: String,
    #[yaserde(attribute)]
    content: String,
}

impl yaserde::YaSerialize for DateTime<Utc> {
    fn serialize<W>(&self, writer: &mut yaserde::ser::Serializer<W>) -> Result<(), String>
    where
        W: std::io::Write,
    {
        writer
            // TODO: make this more robust because this only works if `DateTime` is used as a value
            // of `lastmod` element.
            .write(xml::writer::XmlEvent::start_element("lastmod"))
            .map_err(|e| e.to_string())?;
        writer
            .write(xml::writer::XmlEvent::characters(
                &self.0.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ))
            .map_err(|e| e.to_string())?;
        writer
            .write(xml::writer::XmlEvent::end_element())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn serialize_attributes(
        &self,
        source_attributes: Vec<xml::attribute::OwnedAttribute>,
        source_namespace: xml::namespace::Namespace,
    ) -> Result<
        (
            Vec<xml::attribute::OwnedAttribute>,
            xml::namespace::Namespace,
        ),
        String,
    > {
        Ok((source_attributes, source_namespace))
    }
}

impl yaserde::YaDeserialize for DateTime<Utc> {
    fn deserialize<R: std::io::Read>(
        reader: &mut yaserde::de::Deserializer<R>,
    ) -> Result<Self, String> {
        loop {
            match reader.next_event()? {
                xml::reader::XmlEvent::StartElement { .. } => {}
                xml::reader::XmlEvent::Characters(ref text_content) => {
                    return ChronoDateTime::parse_from_rfc3339(text_content)
                        .map_err(|e| format!("failed to deserialize `{text_content}`: {e}"))
                        .map(|dt| DateTime(dt.with_timezone(&Utc)));
                }
                _ => {
                    break;
                }
            }
        }
        Err("Unable to parse".to_string())
    }
}

impl yaserde::YaSerialize for Url {
    fn serialize<W>(&self, writer: &mut yaserde::ser::Serializer<W>) -> Result<(), String>
    where
        W: std::io::Write,
    {
        writer
            // TODO: make this more robust because this only works if `Url` is used as a value
            // of `loc` element.
            .write(xml::writer::XmlEvent::start_element("loc"))
            .map_err(|e| e.to_string())?;
        writer
            .write(xml::writer::XmlEvent::characters(self.0.as_str()))
            .map_err(|e| e.to_string())?;
        writer
            .write(xml::writer::XmlEvent::end_element())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn serialize_attributes(
        &self,
        source_attributes: Vec<xml::attribute::OwnedAttribute>,
        source_namespace: xml::namespace::Namespace,
    ) -> Result<
        (
            Vec<xml::attribute::OwnedAttribute>,
            xml::namespace::Namespace,
        ),
        String,
    > {
        Ok((source_attributes, source_namespace))
    }
}

impl yaserde::YaDeserialize for Url {
    fn deserialize<R: std::io::Read>(
        reader: &mut yaserde::de::Deserializer<R>,
    ) -> Result<Self, String> {
        loop {
            match reader.next_event()? {
                xml::reader::XmlEvent::StartElement { .. } => {}
                xml::reader::XmlEvent::Characters(ref text_content) => {
                    return Ok(Url(UrlUrl::parse(text_content).map_err(|e| e.to_string())?));
                }
                _ => {
                    break;
                }
            }
        }
        Err("Unable to parse".to_string())
    }
}

async fn website_urls(website: UrlUrl) -> Result<Vec<UrlUrl>, String> {
    let mut urls = vec![];
    let mut website: Website = Website::new(website.as_str());

    website.scrape().await;

    for page in website.get_pages().unwrap().iter() {
        urls.push(UrlUrl::parse(page.get_url().clone()).unwrap())
    }

    Ok(urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_and_deserialize() {
        let str_representation = r#"<?xml version="1.0" encoding="utf-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>1970-01-01T00:01:01Z</lastmod>
    <xhtml:meta name="auto_sitemap_hash" content="1234567890" />
  </url>
</urlset>"#;

        let sitemap = Sitemap {
            pages: vec![Page {
                loc: Some(Url(UrlUrl::parse("https://example.com").unwrap())),
                lastmod: Some(DateTime(ChronoDateTime::<Utc>::from_utc(
                    chrono::NaiveDateTime::from_timestamp_opt(61, 0).unwrap(),
                    Utc,
                ))),
                meta: Some(Meta {
                    name: "auto_sitemap_hash".into(),
                    content: "1234567890".into(),
                }),
            }],
        };

        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };
        let serialized = yaserde::ser::to_string_with_config(&sitemap, &yaserde_cfg).unwrap();
        let deserialized: Sitemap = yaserde::de::from_str(str_representation).unwrap();

        pretty_assertions::assert_eq!(serialized, str_representation);
        pretty_assertions::assert_eq!(deserialized, sitemap);
    }

    mod website_urls {
        use super::*;
        use axum::response::Html;
        use axum::{routing::get, Router};
        use std::net::SocketAddr;

        #[tokio::test]
        async fn test_website_urls() {
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

            let urls = website_urls(UrlUrl::parse("http://localhost:3000").unwrap())
                .await
                .unwrap();

            // Shut down the server...
            let _ = tx.send(());

            let correct_urls = vec![
                UrlUrl::parse("http://localhost:3000/").unwrap(),
                UrlUrl::parse("http://localhost:3000/a").unwrap(),
                UrlUrl::parse("http://localhost:3000/b").unwrap(),
                UrlUrl::parse("http://localhost:3000/c").unwrap(),
                // Shouldn't be reachable by crawling:
                // UrlUrl::parse("http://localhost:3000/d").unwrap(),
            ];

            pretty_assertions::assert_eq!(urls, correct_urls);
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
}
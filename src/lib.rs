use chrono::{DateTime as ChronoDateTime, Utc};
use spider::website::Website;
use url::Url;
use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, PartialEq)]
struct DateTime<Tz: chrono::TimeZone>(ChronoDateTime<Tz>);

#[derive(Debug, PartialEq, Clone)]
struct UrlSerde(Url);

/// Sitemap of the website.
#[derive(Debug, PartialEq)]
pub struct Sitemap {
    pages: Vec<Page>,
}

impl TryFrom<SitemapSerde> for Sitemap {
    type Error = String;
    fn try_from(sitemap_serde: SitemapSerde) -> Result<Self, Self::Error> {
        let pages = sitemap_serde
            .pages
            .into_iter()
            .map(|page| page.try_into())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { pages })
    }
}

impl Sitemap {
    /// Assumes that the URL is domain name.
    pub async fn try_from_url(website_url: Url) -> Result<Self, String> {
        let pages = Self::website_pages(website_url).await?;
        Ok(Self { pages })
    }

    /// Assumes that the string is domain name URL.
    pub async fn try_from_url_str(url_str: &str) -> Result<Self, String> {
        let url = Url::parse(url_str).map_err(|e| format!("failed to parse URL: {}", e))?;
        Self::try_from_url(url).await
    }

    async fn website_pages(website_url: Url) -> Result<Vec<Page>, String> {
        let mut pages = vec![];
        let mut website: Website = Website::new(website_url.as_str());

        website.scrape().await;

        for page in website.get_pages().unwrap().iter() {
            let url = Url::parse(page.get_url()).map_err(|e| e.to_string())?;
            let contents = page.get_html();
            let hash = md5::compute(contents);
            pages.push(Page {
                url,
                lastmod: Some(DateTime(chrono::Utc::now())),
                md5_hash: Some(format!("{:x}", hash)),
            });
        }

        Ok(pages)
    }

    pub fn deserialize<R: std::io::Read>(reader: R) -> Result<Self, String> {
        let sitemap_serde: SitemapSerde = yaserde::de::from_reader(reader)
            .map_err(|e| format!("failed to deserialize: {}", e))?;

        Self::try_from(sitemap_serde)
    }

    pub fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), String> {
        let sitemap_serde: SitemapSerde = self.into();

        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };
        yaserde::ser::serialize_with_writer(&sitemap_serde, writer, &yaserde_cfg)
            .map_err(|e| format!("failed to serialize: {}", e))?;

        Ok(())
    }

    pub fn sort_by_url(&mut self) {
        self.pages.sort_by(|a, b| a.url.cmp(&b.url));
    }

    /// Ignores pages that are missing in the new sitemap.
    /// Uses the old `lastmod` if the hash unchanged, otherwise uses the new `lastmod`.
    pub fn combine_with_old_sitemap(&mut self, old_sitemap: &Sitemap) -> Result<(), String> {
        // HashMap of old URLs and the corresponding `Page`.
        let old_pages = old_sitemap
            .pages
            .iter()
            .map(|page| (page.url.clone(), page))
            .collect::<std::collections::HashMap<_, _>>();

        for page in self.pages.iter_mut() {
            if let Some(old_page) = old_pages.get(&page.url) {
                if let (Some(old_hash), Some(old_lastmod)) = (
                    old_page.md5_hash.clone(),
                    old_page.lastmod.as_ref().map(|x| DateTime(x.0)),
                ) {
                    if Some(old_hash) == page.md5_hash {
                        page.lastmod = Some(old_lastmod);
                        continue;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Page {
    url: Url,
    lastmod: Option<DateTime<Utc>>,
    md5_hash: Option<String>,
}

impl TryFrom<PageSerde> for Page {
    type Error = String;

    fn try_from(page_serde: PageSerde) -> Result<Self, Self::Error> {
        let hash = match page_serde.meta {
            Some(meta) => {
                if meta.name == "auto_sitemap_md5_hash" && meta.content.len() == 32 {
                    Some(meta.content)
                } else {
                    None
                }
            }
            None => None,
        };
        Ok(Self {
            url: page_serde
                .url
                .ok_or_else(|| "page URL is missing".to_string())?
                .0,
            lastmod: page_serde.lastmod,
            md5_hash: hash,
        })
    }
}

#[derive(Debug, PartialEq, Clone, YaSerialize, YaDeserialize)]
#[yaserde(namespace = "xhtml: http://www.w3.org/1999/xhtml")]
pub struct Meta {
    #[yaserde(attribute)]
    name: String,
    #[yaserde(attribute)]
    content: String,
}

#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
#[yaserde(
    rename = "urlset",
    namespace = "http://www.sitemaps.org/schemas/sitemap/0.9"
    namespace = "xhtml: http://www.w3.org/1999/xhtml"
)]
struct SitemapSerde {
    #[yaserde(rename = "url")]
    pages: Vec<PageSerde>,
}

impl From<&Sitemap> for SitemapSerde {
    fn from(sitemap: &Sitemap) -> Self {
        let pages = sitemap
            .pages
            .iter()
            .map(|page| page.into())
            .collect::<Vec<_>>();
        SitemapSerde { pages }
    }
}

#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
struct PageSerde {
    #[yaserde(rename = "loc")]
    url: Option<UrlSerde>,
    lastmod: Option<DateTime<Utc>>,
    #[yaserde(prefix = "xhtml")]
    meta: Option<Meta>,
}

impl From<&Page> for PageSerde {
    fn from(page: &Page) -> Self {
        let meta = page.md5_hash.as_ref().map(|hash| Meta {
            name: "auto_sitemap_md5_hash".to_string(),
            content: hash.clone(),
        });
        Self {
            url: Some(UrlSerde(page.url.clone())),
            lastmod: page.lastmod.as_ref().map(|lastmod| DateTime(lastmod.0)),
            meta,
        }
    }
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

impl yaserde::YaSerialize for UrlSerde {
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

impl yaserde::YaDeserialize for UrlSerde {
    fn deserialize<R: std::io::Read>(
        reader: &mut yaserde::de::Deserializer<R>,
    ) -> Result<Self, String> {
        loop {
            match reader.next_event()? {
                xml::reader::XmlEvent::StartElement { .. } => {}
                xml::reader::XmlEvent::Characters(ref text_content) => {
                    return Ok(UrlSerde(
                        Url::parse(text_content).map_err(|e| e.to_string())?,
                    ));
                }
                _ => {
                    break;
                }
            }
        }
        Err("Unable to parse".to_string())
    }
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
    <xhtml:meta name="auto_sitemap_md5_hash" content="0123456789abcdef0123456789abcdef" />
  </url>
</urlset>"#;

        let sitemap = Sitemap {
            pages: vec![Page {
                url: Url::parse("https://example.com").unwrap(),
                lastmod: Some(DateTime(ChronoDateTime::<Utc>::from_utc(
                    chrono::NaiveDateTime::from_timestamp_opt(61, 0).unwrap(),
                    Utc,
                ))),
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
                let lastmod = page
                    .lastmod
                    .as_ref()
                    .map(|lastmod| DateTime(lastmod.0))
                    .unwrap();

                // Lastmod should be updated to less than 1 second after `start_time`.
                if updated_urls.contains(&page.url) {
                    assert!(lastmod.0 > start_time);
                    assert!(lastmod.0 < start_time + chrono::Duration::seconds(1));
                } else {
                    assert!(lastmod.0 < start_time);
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
}

use chrono::{DateTime as ChronoDateTime, Utc};
use url::Url as UrlUrl;
use yaserde_derive::YaSerialize;

struct DateTime<Tz: chrono::TimeZone>(ChronoDateTime<Tz>);

struct Url(UrlUrl);

/// Sitemap of the website.
#[derive(YaSerialize)]
#[yaserde(
    rename = "urlset",
    namespace = "http://www.sitemaps.org/schemas/sitemap/0.9"
    namespace = "xhtml: http://www.w3.org/1999/xhtml"
)]
struct Sitemap {
    #[yaserde(rename = "url")]
    pages: Vec<Page>,
}

#[derive(YaSerialize)]
struct Page {
    loc: Url,
    lastmod: DateTime<Utc>,
    #[yaserde(prefix = "xhtml")]
    meta: Option<Meta>,
}

#[derive(YaSerialize)]
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

#[test]
fn test() {
    use pretty_assertions::assert_eq;

    let sitemap = Sitemap {
        pages: vec![Page {
            loc: Url(UrlUrl::parse("https://example.com").unwrap()),
            lastmod: DateTime(ChronoDateTime::<Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp_opt(61, 0).unwrap(),
                Utc,
            )),
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

    assert_eq!(
        serialized,
        r#"<?xml version="1.0" encoding="utf-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>1970-01-01T00:01:01Z</lastmod>
    <xhtml:meta name="auto_sitemap_hash" content="1234567890" />
  </url>
</urlset>"#,
    );
}

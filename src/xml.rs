use chrono::{DateTime, Utc};
use url::Url;
use yaserde_derive::{YaDeserialize, YaSerialize};

use crate::{Page, Sitemap};

#[derive(Debug, PartialEq)]
pub struct DateTimeSerde<Tz: chrono::TimeZone>(pub DateTime<Tz>);

impl From<DateTimeSerde<Utc>> for DateTime<Utc> {
    fn from(val: DateTimeSerde<Utc>) -> Self {
        val.0
    }
}

impl From<DateTime<Utc>> for DateTimeSerde<Utc> {
    fn from(val: DateTime<Utc>) -> Self {
        DateTimeSerde(val)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct UrlSerde(Url);

impl From<UrlSerde> for Url {
    fn from(val: UrlSerde) -> Self {
        val.0
    }
}

#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
#[yaserde(
    rename = "urlset",
    namespace = "http://www.sitemaps.org/schemas/sitemap/0.9"
    namespace = "xhtml: http://www.w3.org/1999/xhtml"
)]
pub struct SitemapSerde {
    #[yaserde(rename = "url")]
    pub pages: Vec<PageSerde>,
}

#[derive(Debug, PartialEq, YaSerialize, YaDeserialize)]
pub struct PageSerde {
    #[yaserde(rename = "loc")]
    pub url: Option<UrlSerde>,
    pub lastmod: Option<DateTimeSerde<Utc>>,
    #[yaserde(prefix = "xhtml")]
    pub meta: Vec<Meta>,
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

impl From<&Page> for PageSerde {
    fn from(page: &Page) -> Self {
        let meta = page.md5_hash.as_ref().map(|hash| Meta {
            name: "auto_sitemap_md5_hash".to_string(),
            content: hash.clone(),
        });
        Self {
            url: Some(UrlSerde(page.url.clone())),
            lastmod: page.lastmod.map(|lastmod| lastmod.into()),
            meta: meta.into_iter().collect(),
        }
    }
}

impl TryFrom<PageSerde> for Page {
    type Error = String;

    fn try_from(page_serde: PageSerde) -> Result<Self, Self::Error> {
        let hash = page_serde.meta.into_iter().find_map(|meta| {
            let name = meta.name.trim();
            let content = meta.content.trim();
            if name == "auto_sitemap_md5_hash" && content.len() == 32 {
                Some(content.to_string())
            } else {
                None
            }
        });
        Ok(Self {
            url: page_serde
                .url
                .ok_or_else(|| "page URL is missing".to_string())?
                .into(),
            lastmod: page_serde.lastmod.map(|lastmod| lastmod.into()),
            md5_hash: hash,
        })
    }
}

impl yaserde::YaSerialize for DateTimeSerde<Utc> {
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

impl yaserde::YaDeserialize for DateTimeSerde<Utc> {
    fn deserialize<R: std::io::Read>(
        reader: &mut yaserde::de::Deserializer<R>,
    ) -> Result<Self, String> {
        loop {
            match reader.next_event()? {
                xml::reader::XmlEvent::StartElement { .. } => {}
                xml::reader::XmlEvent::Characters(ref text_content) => {
                    return DateTime::parse_from_rfc3339(text_content)
                        .map_err(|e| format!("failed to deserialize `{text_content}`: {e}"))
                        .map(|dt| DateTimeSerde(dt.with_timezone(&Utc)));
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

#[derive(Debug, PartialEq, Clone, YaSerialize, YaDeserialize)]
#[yaserde(namespace = "xhtml: http://www.w3.org/1999/xhtml")]
pub struct Meta {
    #[yaserde(attribute)]
    name: String,
    #[yaserde(attribute)]
    content: String,
}

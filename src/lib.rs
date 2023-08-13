use chrono::{DateTime, Utc};
use spider::website::Website;
use url::Url;
use yaserde_derive::{YaDeserialize, YaSerialize};

mod xml;
use crate::xml::SitemapSerde;

/// Sitemap of the website.
#[derive(Debug, PartialEq)]
pub struct Sitemap {
    pub pages: Vec<Page>,
}

impl Sitemap {
    /// Assumes that the URL is domain name.
    pub async fn try_from_url(website_url: Url) -> Result<Self, String> {
        let mut pages = vec![];
        let mut website: Website = Website::new(website_url.as_str());

        website.scrape().await;

        for page in website.get_pages().unwrap().iter() {
            let url = Url::parse(page.get_url()).map_err(|e| e.to_string())?;
            let contents = page.get_html().trim().replace("\r\n", "\n"); // normalize line endings
            let hash = md5::compute(contents);
            pages.push(Page {
                url,
                lastmod: Some(chrono::Utc::now()),
                md5_hash: Some(format!("{:x}", hash)),
            });
        }

        Ok(Self { pages })
    }

    /// Assumes that the string is domain name URL.
    pub async fn try_from_url_str(url_str: &str) -> Result<Self, String> {
        let url = Url::parse(url_str).map_err(|e| format!("failed to parse URL: {}", e))?;
        Self::try_from_url(url).await
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
                if let (Some(old_hash), Some(old_lastmod)) =
                    (old_page.md5_hash.clone(), old_page.lastmod)
                {
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
    pub url: Url,
    pub lastmod: Option<DateTime<Utc>>,
    pub md5_hash: Option<String>,
}

#[derive(Debug, PartialEq, Clone, YaSerialize, YaDeserialize)]
#[yaserde(namespace = "xhtml: http://www.w3.org/1999/xhtml")]
pub struct Meta {
    #[yaserde(attribute)]
    pub name: String,
    #[yaserde(attribute)]
    pub content: String,
}

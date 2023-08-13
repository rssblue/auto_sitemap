#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use chrono::{DateTime, Utc};
use spider::website::Website;
use url::Url;

mod xml;
use crate::xml::SitemapSerde;

/// Sitemap of the website.
#[derive(Debug, PartialEq)]
pub struct Sitemap {
    /// Pages of the website.
    pub pages: Vec<Page>,
}

impl Sitemap {
    /// Generates sitemap by crawling the website.
    pub async fn generate_by_crawling(website_url: impl AsRef<str>) -> Result<Self, String> {
        let website_url = Url::parse(website_url.as_ref()).map_err(|e| e.to_string())?;
        if website_url.scheme() != "http" && website_url.scheme() != "https" {
            return Err("URL should start with http:// or https://".to_string());
        }

        let mut pages = vec![];
        let mut website: Website = Website::new(website_url.as_str());

        website.scrape().await;

        let spider_pages = match website.get_pages() {
            Some(spider_pages) => spider_pages,
            None => return Err("failed to get pages".to_string()),
        };

        for page in spider_pages.iter() {
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

    /// Imports sitemap from URL or local file.
    pub async fn import(url_or_filepath: impl AsRef<str>) -> Result<Self, String> {
        let url_or_filepath = url_or_filepath.as_ref();
        if url_or_filepath.starts_with("http://") || url_or_filepath.starts_with("https://") {
            Self::import_from_url(url_or_filepath).await
        } else {
            Self::import_from_file(url_or_filepath)
        }
    }

    /// Imports sitemap from URL.
    async fn import_from_url(url: &str) -> Result<Self, String> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("failed to get {}: {}", url, e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("failed to get {}: {}", url, e))?;
        let sitemap = Self::deserialize(&bytes[..])?;

        Ok(sitemap)
    }

    /// Imports sitemap from local file.
    fn import_from_file(filepath: &str) -> Result<Self, String> {
        let file = std::fs::File::open(filepath)
            .map_err(|e| format!("failed to open {}: {}", filepath, e))?;

        let sitemap = Self::deserialize(file)?;

        Ok(sitemap)
    }

    /// Deserializes from XML sitemap.
    /// Additional fields are ignored.
    pub fn deserialize<R: std::io::Read>(reader: R) -> Result<Self, String> {
        let sitemap_serde: SitemapSerde = yaserde::de::from_reader(reader)
            .map_err(|e| format!("failed to deserialize: {}", e))?;

        Self::try_from(sitemap_serde)
    }

    /// Serializes to XML sitemap.
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

    /// Sorts pages by URL.
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

/// Page of the website.
#[derive(Debug, PartialEq)]
pub struct Page {
    /// Page URL.
    pub url: Url,
    /// Last modification date.
    pub lastmod: Option<DateTime<Utc>>,
    /// MD5 hash of the page contents.
    /// Used to detect changes.
    pub md5_hash: Option<String>,
}

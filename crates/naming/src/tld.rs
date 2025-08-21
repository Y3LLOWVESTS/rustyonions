use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TldType {
    // Media
    Image,
    Video,
    Audio,
    // Text/social
    Post,
    Comment,
    News,
    Journalist,
    Blog,
    // Maps/data
    Map,
    Route,
    // Identity / new direction
    Passport, // replaces "sso"
}

impl TldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TldType::Image => "image",
            TldType::Video => "video",
            TldType::Audio => "audio",
            TldType::Post => "post",
            TldType::Comment => "comment",
            TldType::News => "news",
            TldType::Journalist => "journalist",
            TldType::Blog => "blog",
            TldType::Map => "map",
            TldType::Route => "route",
            TldType::Passport => "passport",
        }
    }
}

impl fmt::Display for TldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error)]
pub enum TldParseError {
    #[error("unknown tld: {0}")]
    Unknown(String),
}

impl FromStr for TldType {
    type Err = TldParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().trim_start_matches('.');
        Ok(match s {
            "image" => Self::Image,
            "video" => Self::Video,
            "audio" => Self::Audio,
            "post" => Self::Post,
            "comment" => Self::Comment,
            "news" => Self::News,
            "journalist" => Self::Journalist,
            "blog" => Self::Blog,
            "map" => Self::Map,
            "route" => Self::Route,
            "passport" => Self::Passport,
            other => return Err(TldParseError::Unknown(other.to_string())),
        })
    }
}

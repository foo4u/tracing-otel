use http::Version;

pub(crate) struct HttpVersion {
    pub protocol: String,
    pub version: String,
}

impl From<Version> for HttpVersion {
    fn from(version: Version) -> Self {
        let protocol = "HTTP".to_string();
        match version {
            Version::HTTP_09 => HttpVersion {
                protocol,
                version: "0.9".into(),
            },
            Version::HTTP_10 => HttpVersion {
                protocol,
                version: "1.0".into(),
            },
            Version::HTTP_11 => HttpVersion {
                protocol,
                version: "1.1".into(),
            },
            Version::HTTP_2 => HttpVersion {
                protocol,
                version: "2".into(),
            },
            Version::HTTP_3 => HttpVersion {
                protocol,
                version: "3".into(),
            },
            _ => HttpVersion {
                protocol,
                version: "unknown".into(),
            },
        }
    }
}

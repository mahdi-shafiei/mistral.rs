use std::{
    fs::{self, File},
    io::Read,
};

use image::DynamicImage;
use pyo3::{exceptions::PyValueError, PyErr};

pub(crate) struct PyApiErr(pub(crate) PyErr);
pub(crate) type PyApiResult<T> = Result<T, PyApiErr>;

impl std::fmt::Debug for PyApiErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for PyApiErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for PyApiErr {}

impl From<reqwest::Error> for PyApiErr {
    fn from(value: reqwest::Error) -> Self {
        Self::from(value.to_string())
    }
}

impl From<std::io::Error> for PyApiErr {
    fn from(value: std::io::Error) -> Self {
        Self::from(value.to_string())
    }
}

impl From<anyhow::Error> for PyApiErr {
    fn from(value: anyhow::Error) -> Self {
        Self::from(value.to_string())
    }
}

impl From<serde_json::Error> for PyApiErr {
    fn from(value: serde_json::Error) -> Self {
        Self::from(value.to_string())
    }
}

impl From<mistralrs_core::MistralRsError> for PyApiErr {
    fn from(value: mistralrs_core::MistralRsError) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for PyApiErr {
    fn from(value: String) -> Self {
        Self(PyValueError::new_err(value.to_string()))
    }
}

impl From<&str> for PyApiErr {
    fn from(value: &str) -> Self {
        Self(PyValueError::new_err(value.to_string()))
    }
}

impl From<PyApiErr> for PyErr {
    fn from(value: PyApiErr) -> Self {
        value.0
    }
}

pub(crate) fn parse_image_url(url_unparsed: &str) -> PyApiResult<DynamicImage> {
    let url = if let Ok(url) = url::Url::parse(url_unparsed) {
        url
    } else if File::open(url_unparsed).is_ok() {
        url::Url::from_file_path(std::path::absolute(url_unparsed)?)
            .map_err(|_| format!("Could not parse file path: {}", url_unparsed))?
    } else {
        url::Url::parse(&format!("data:image/png;base64,{}", url_unparsed))
            .map_err(|_| format!("Could not parse as base64 data: {}", url_unparsed))?
    };

    let bytes = if url.scheme() == "http" || url.scheme() == "https" {
        // Read from http
        match reqwest::blocking::get(url.clone()) {
            Ok(http_resp) => http_resp.bytes()?.to_vec(),
            Err(e) => return Err(PyApiErr::from(format!("{e}"))),
        }
    } else if url.scheme() == "file" {
        let path = url
            .to_file_path()
            .map_err(|_| format!("Could not parse file path: {}", url))?;

        if let Ok(mut f) = File::open(&path) {
            // Read from local file
            let metadata = fs::metadata(&path)?;
            let mut buffer = vec![0; metadata.len() as usize];
            f.read_exact(&mut buffer)?;
            buffer
        } else {
            return Err(PyApiErr::from(format!(
                "Could not open file at path: {}",
                url
            )));
        }
    } else if url.scheme() == "data" {
        // Decode with base64
        let data_url = data_url::DataUrl::process(url.as_str()).map_err(|e| format!("{e}"))?;
        data_url.decode_to_vec().map_err(|e| format!("{e}"))?.0
    } else {
        return Err(PyApiErr::from(format!(
            "Unsupported URL scheme: {}",
            url.scheme()
        )));
    };

    image::load_from_memory(&bytes).map_err(|e| PyApiErr::from(format!("{e}")))
}
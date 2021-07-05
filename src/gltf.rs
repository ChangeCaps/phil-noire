use gltf::{buffer::Source, Gltf};

#[derive(Clone, Debug)]
pub enum GltfError {
    MissingPositions,
    MissingBlob,
}

impl std::fmt::Display for GltfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for GltfError {}

pub fn load_buffers(gltf: &Gltf) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut buffer_data = Vec::new();

    for buffer in gltf.buffers() {
        match buffer.source() {
            Source::Uri(uri) => {
                let buffer_bytes = match DataUri::parse(uri) {
                    Ok(data_uri) => data_uri.decode()?,
                    Err(()) => return Err(GltfError::MissingBlob.into()),
                };

                buffer_data.push(buffer_bytes);
            }
            Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffer_data.push(blob.into());
                } else {
                    return Err(GltfError::MissingBlob.into());
                }
            }
        }
    }

    Ok(buffer_data)
}

struct DataUri<'a> {
    mime_type: &'a str,
    base64: bool,
    data: &'a str,
}

impl<'a> DataUri<'a> {
    fn parse(uri: &'a str) -> Result<Self, ()> {
        let uri = uri.strip_prefix("data:").ok_or(())?;
        let mut iter = uri.splitn(2, ',');
        let mime_type = iter.next().ok_or(())?;
        let uri = iter.next().ok_or(())?;

        let (mime_type, base64) = match mime_type.strip_suffix(";base64") {
            Some(mime_type) => (mime_type, true),
            None => (mime_type, false),
        };

        Ok(Self {
            mime_type,
            base64,
            data: uri,
        })
    }

    fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        if self.base64 {
            base64::decode(self.data)
        } else {
            Ok(self.data.as_bytes().to_owned())
        }
    }
}

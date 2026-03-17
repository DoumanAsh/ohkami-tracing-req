use core::{ptr, cmp, fmt};

type RequestIdBuffer = [u8; 64];

#[derive(Clone)]
///Request's id
///
///By default it is extracted from `X-Request-Id` header
pub struct RequestId {
    buffer: RequestIdBuffer,
    len: u8,
}

impl RequestId {
    pub(crate) fn from_str(bytes: &str) -> Self {
        let mut buffer: RequestIdBuffer = [0; 64];

        let len = cmp::min(buffer.len(), bytes.len());

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.as_mut_ptr(), len)
        };

        Self {
            buffer,
            len: len as _,
        }
    }

    ///Initializes itself from `uuid`
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        let mut buffer: RequestIdBuffer = [0; 64];
        let uuid = uuid.as_hyphenated();
        let len = uuid.encode_lower(&mut buffer).len();

        Self {
            buffer,
            len: len as _,
        }
    }

    #[inline]
    ///Returns slice to already written data.
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.buffer.as_ptr(), self.len as _)
        }
    }

    #[inline(always)]
    ///Gets textual representation of the request id, if header value is string
    pub const fn as_str(&self) -> &str {
        match core::str::from_utf8(self.as_bytes()) {
            Ok(header) => header,
            Err(_) => unreachable!(),
        }
    }
}

impl fmt::Debug for RequestId {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), fmt)
    }
}

impl fmt::Display for RequestId {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), fmt)
    }
}

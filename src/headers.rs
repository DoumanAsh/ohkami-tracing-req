use core::fmt;

///RequestId's header name (lower case)
pub const REQUEST_ID_LOW: &str = "x-request-id";
///RequestId's header name
pub const REQUEST_ID: &str = "X-Request-Id";

pub struct InspectHeaders<'a> {
    pub header_list: &'a [&'a str],
    pub headers: &'a ohkami::request::RequestHeaders,
}

impl fmt::Debug for InspectHeaders<'_> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = fmt.debug_map();
        for key in self.header_list {
            if let Some(value) = self.headers.get(key) {
                out.entry(&key, &value);
            }
        }

        out.finish()
    }
}

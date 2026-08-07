use reqwest::RequestBuilder;

/// Common request context
pub struct ApiContext<'a> {
    pub bearer_token: &'a str,
    pub org_id: &'a str,
}

/// The request context must provide Org info
pub trait OrgScoped {
    fn org(&self) -> &str;
}

impl<'a> OrgScoped for ApiContext<'a> {
    fn org(&self) -> &str {
        self.org_id
    }
}

/// Append the Org-related header to the request
pub fn apply_org_header<T>(request: RequestBuilder, org_scoped: &T) -> RequestBuilder
where
    T: OrgScoped + ?Sized,
{
    request.header("X-JMS-ORG", org_scoped.org())
}

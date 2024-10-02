use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Context {
    c_token: CancellationToken,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        let c_token = CancellationToken::new();
        Context { c_token }
    }

    pub fn token(&self) -> CancellationToken {
        self.c_token.clone()
    }

    pub fn cancel(&self) {
        self.c_token.cancel();
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            c_token: self.c_token.clone(),
        }
    }
}

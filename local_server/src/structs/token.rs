#[derive(Debug)]
pub struct Token {
    status: bool,
}

impl Token {
    pub fn new() -> Self {
        let status: bool = false;

        Self { status }
    }

    pub fn is_avaliable(&self) -> bool {
        self.status
    }


    pub fn avaliable(&mut self) {
        self.status = true;
    }

    pub fn not_avaliable(&mut self) {
        self.status = false;
    }

}

impl Default for Token {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod token_test {
    use super::Token;

    #[test]
    fn test01_token_start_not_avalible() {
        let token = Token::new();

        assert!(!token.is_avaliable());
    }

    #[test]
    fn test02_token_avaliable_is_avalible() {
        let mut token = Token::new();
        token.avaliable();

        assert!(token.is_avaliable());
    }

    #[test]
    fn test03_token_not_avaliable_is_not_avalible() {
        let mut token = Token::new();
        token.not_avaliable();

        assert!(!token.is_avaliable());
    }
}

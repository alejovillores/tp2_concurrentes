#[derive(Debug)]
pub struct Token {
    status: bool,
    cont: u32,
}

impl Token {
    pub fn new() -> Self {
        let status: bool = false;
        let cont = 0;

        Self { status, cont }
    }

    pub fn is_avaliable(&self) -> bool {
        self.status
    }

    pub fn cont(&self) -> u32 {
        self.cont
    }

    pub fn avaliable(&mut self) {
        self.status = true;
    }

    pub fn not_avaliable(&mut self) {
        self.status = false;
    }

    pub fn increase(&mut self) {
        self.cont += 1;
    }

    pub fn decrease(&mut self) {
        self.cont -= 1;
    }

    pub fn empty(&self) -> bool {
        self.cont == 0
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
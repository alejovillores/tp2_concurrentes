#[allow(dead_code)]
#[derive(Debug)]
pub struct Account {
    pub customer_id: u32,
    pub points: u32,
    pub blocked_points: u32,
    pub points_to_add: u32,
}

impl Account {
    pub fn new(customer_id: u32) -> Result<Account, String> {
        Ok(Self {
            customer_id,
            points: 0,
            blocked_points: 0,
            points_to_add: 0,
        })
    }

    pub fn add_points(&mut self, points: u32) {
        self.points_to_add += points;
    }

    pub fn register_added_points(&mut self) {
        self.points += self.points_to_add;
        self.points_to_add = 0;
    }

    pub fn subtract_points(&mut self, points: u32) -> Result<(), String> {
        if self.blocked_points >= points {
            self.blocked_points -= points;
            if self.points_to_add > points {
                self.points_to_add -= points;
            } else {
                self.points -= points;
            }
            Ok(())
        } else {
            Err("No se han bloqueado los puntos con anterioridad".to_string())
        }
    }

    pub fn block_points(&mut self, points: u32) -> Result<(), String> {
        if (self.points + self.points_to_add) - self.blocked_points >= points {
            self.blocked_points += points;
            Ok(())
        } else {
            Err("No hay suficientes puntos disponibles".to_string())
        }
    }

    pub fn unblock_points(&mut self, points: u32) -> Result<(), String> {
        if self.blocked_points >= points {
            self.blocked_points -= points;
            Ok(())
        } else {
            Err("No hay suficientes puntos bloqueados".to_string())
        }
    }

    pub fn sync(&mut self, points: u32) -> Result<(), String> {
        self.points = points;
        Ok(())
    }
}

#[cfg(test)]
mod account_test {
    use super::*;

    #[test]
    fn test_add_points() {
        let mut account = Account::new(123).unwrap();
        account.add_points(15);
        assert_eq!(account.points, 15);
        account.add_points(10);
        assert_eq!(account.points, 25);
    }

    #[test]
    fn test_subtract_points_with_enough_blocked_points_success() {
        let mut account = Account::new(123).unwrap();
        account.points = 30;
        account.blocked_points = 15;
        let result = account.subtract_points(10);
        assert_eq!(account.points, 20);
        assert_eq!(account.blocked_points, 5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_subtract_points_with_not_enough_blocked_points_fails() {
        let mut account = Account::new(123).unwrap();
        account.points = 30;
        account.blocked_points = 5;
        let result = account.subtract_points(10);
        assert_eq!(account.points, 30);
        assert_eq!(account.blocked_points, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_block_points_with_enough_points_success() {
        let mut account = Account::new(123).unwrap();
        account.points = 15;
        let result = account.block_points(10);
        assert_eq!(account.points, 15);
        assert_eq!(account.blocked_points, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_points_with_enough_left_points_fails() {
        let mut account = Account::new(123).unwrap();
        account.points = 15;
        let _ = account.block_points(10);
        let result = account.block_points(10);
        assert_eq!(account.points, 15);
        assert_eq!(account.blocked_points, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_block_points_with_not_enough_points_fails() {
        let mut account = Account::new(123).unwrap();
        account.points = 5;
        let result = account.block_points(10);
        assert_eq!(account.points, 5);
        assert_eq!(account.blocked_points, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_unblock_points_with_enough_blocked_points_success() {
        let mut account = Account::new(123).unwrap();
        account.points = 15;
        account.blocked_points = 10;
        let result = account.unblock_points(10);
        assert_eq!(account.points, 15);
        assert_eq!(account.blocked_points, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unblock_points_with_not_enough_blocked_points_fails() {
        let mut account = Account::new(123).unwrap();
        account.points = 15;
        account.blocked_points = 5;
        let result = account.unblock_points(10);
        assert_eq!(account.points, 15);
        assert_eq!(account.blocked_points, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_account_success() {
        let mut account = Account::new(123).unwrap();
        let points = 20;
        let result = account.sync(points);
        assert_eq!(account.points, 20);
        assert_eq!(account.blocked_points, 0);
        assert!(result.is_ok());
    }
}

#[allow(dead_code)]
pub struct Account {
    customer_id: u32,
    points: u32,
    blocked_points: u32,
}

impl Account {
    pub fn new(customer_id: u32) -> Result<Account, String>{
        Ok(Self{
            customer_id,
            points: 0,
            blocked_points: 0,
        })
    }

    pub fn add_points(&mut self, points: u32) {
        self.points += points;
    }

    pub fn subtract_points(&mut self, points: u32) -> Result<(), String> {
        if self.blocked_points >= points {
            self.blocked_points -= points;
            self.points -= points;
            Ok(())
        } else {
            Err("No se han bloqueado los puntos con anterioridad". to_string())
        }
    }

    pub fn block_points(&mut self, points: u32) -> Result<(), String> {
        if self.points >= points {
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
}

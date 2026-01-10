use common::registers::{DriveState, DriveAction};

pub struct GaragePhysics {
    pub current_position: f32, // 0.0 to 200.0 (0 = Closed, 200 = Open)
    pub target_position: f32,
    pub light_on: bool,
    pub vent_on: bool,
    pub speed: f32, // Position units per tick
}

impl GaragePhysics {
    pub fn new() -> Self {
        Self {
            current_position: 0.0, // Start closed
            target_position: 0.0,
            light_on: false,
            vent_on: false,
            speed: 1.0, // Adjust based on poll frequency
        }
    }

    pub fn tick(&mut self) {
        if (self.current_position - self.target_position).abs() < self.speed {
            self.current_position = self.target_position;
        } else if self.current_position < self.target_position {
            self.current_position += self.speed;
        } else {
            self.current_position -= self.speed;
        }
    }

    pub fn get_drive_state(&self) -> DriveState {
        let diff = (self.current_position - self.target_position).abs();
        let moving = diff > 0.1;

        if moving {
            if self.target_position > self.current_position {
                DriveState::Opening
            } else {
                DriveState::Closing
            }
        } else {
            if self.current_position >= 199.0 {
                DriveState::Open
            } else if self.current_position <= 1.0 {
                DriveState::Closed
            } else if (self.current_position - 100.0).abs() < 10.0 {
                DriveState::HalfOpenReached
            } else {
                DriveState::Stopped
            }
        }
    }

    pub fn handle_action(&mut self, action: DriveAction) {
        match action {
            DriveAction::Open => self.target_position = 200.0,
            DriveAction::Close => self.target_position = 0.0,
            DriveAction::Stop => self.target_position = self.current_position,
            DriveAction::HalfOpen => self.target_position = 100.0,
            DriveAction::Vent => {
                // Approximate vent position
                self.target_position = 20.0; 
                self.vent_on = true;
            },
            DriveAction::ToggleLight => self.light_on = !self.light_on,
            DriveAction::None => {},
        }
    }
}

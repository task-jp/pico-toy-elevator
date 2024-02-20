use crate::button::LedButtonTrait;
use alloc::boxed::Box;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Point, Size},
    mono_font::{ascii::FONT_10X20, ascii::FONT_5X8, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    primitives::{PrimitiveStyleBuilder, Rectangle, StyledDrawable, Triangle},
    text::Text,
};
use rp_pico::pac::pio0::flevel;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Up(Option<u8>),
    Down(Option<u8>),
    Idle,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DoorState {
    Opening(u8),
    Open(u8),
    Closing(u8),
    Closed,
}

struct Floor {
    number: i8,
    label: &'static str,
    pronunciation: &'static [u8],
    stop: bool,
    button: Box<dyn LedButtonTrait>,
}

pub struct Elevator {
    current_floor_index: usize,
    direction: Direction,
    door: DoorState,
    floors: [Floor; 8],
    repaint: Option<Box<dyn FnMut()>>,
    announce: Option<Box<dyn FnMut(&[u8])>>,
}

impl Elevator {
    pub fn new(floors: [(i8, &'static str, &'static [u8], Box<dyn LedButtonTrait>); 8]) -> Self {
        // find the index of floor 1
        let index = floors.iter().position(|(number, _, _, _)| *number == 1);
        Self {
            current_floor_index: index.unwrap(),
            direction: Direction::Idle,
            door: DoorState::Closed,
            floors: floors.map(|(number, label, pronunciation, button)| Floor {
                number,
                label,
                pronunciation,
                stop: false,
                button,
            }),
            repaint: None,
            announce: None,
        }
    }

    fn set_direction(&mut self, direction: Direction) {
        if self.direction == direction {
            return;
        }
        self.direction = direction;
        if let Some(callback) = &mut self.repaint {
            callback();
        }
    }

    fn set_door(&mut self, door: DoorState) {
        if self.door == door {
            return;
        }
        self.door = door;
        if let Some(callback) = &mut self.repaint {
            callback();
        }
    }

    fn set_current_floor_index(&mut self, index: usize) {
        if self.current_floor_index == index {
            return;
        }
        self.current_floor_index = index;
        if let Some(callback) = &mut self.repaint {
            callback();
        }
    }

    fn goto_next_floor(&mut self) {
        let index = self.current_floor_index;
        let upper_floors = &self.floors[index..];
        let lower_floors = &self.floors[..index];
        let direction = match self.direction {
            Direction::Up(_) => {
                if upper_floors.iter().position(|f| f.stop).is_some() {
                    Direction::Up(Some(0))
                } else if lower_floors.iter().position(|f| f.stop).is_some() {
                    Direction::Down(Some(0))
                } else {
                    Direction::Idle
                }
            }
            Direction::Down(_) => {
                if lower_floors.iter().position(|f| f.stop).is_some() {
                    Direction::Down(Some(0))
                } else if upper_floors.iter().position(|f| f.stop).is_some() {
                    Direction::Up(Some(0))
                } else {
                    Direction::Idle
                }
            }
            Direction::Idle => {
                let upper = upper_floors.iter().position(|f| f.stop);
                let lower = lower_floors.iter().position(|f| f.stop);
                match (upper, lower) {
                    (Some(_), Some(_)) => {
                        if upper.unwrap() - index < index - lower.unwrap() {
                            Direction::Up(Some(0))
                        } else {
                            Direction::Down(Some(0))
                        }
                    }
                    (Some(_), _) => Direction::Up(Some(0)),
                    (_, Some(_)) => Direction::Down(Some(0)),
                    _ => Direction::Idle,
                }
            }
        };

        match direction {
            Direction::Up(_) => {
                if let Some(callback) = &mut self.announce {
                    callback(b"ueni/mairima'_su,\r");
                }
            }
            Direction::Down(_) => {
                if let Some(callback) = &mut self.announce {
                    callback(b"shitani/mairima'_su,\r");
                }
            }
            Direction::Idle => {}
        }
        self.set_direction(direction);
    }

    pub fn advance(&mut self) {
        // check if button is clicked
        for (index, floor) in self.floors.iter_mut().enumerate() {
            if floor.button.is_pressed().unwrap() {
                if !floor.stop {
                    floor.stop = true;
                    floor.button.turn_on().unwrap();
                    if self.direction == Direction::Idle && self.current_floor_index == index {
                        self.set_door(DoorState::Opening(0));
                        return;
                    }
                }
            }
        }
        // while door is moving, do it
        match self.door {
            DoorState::Opening(progress) => {
                match progress {
                    100 => {
                        self.set_door(DoorState::Open(0));
                    }
                    0 => {
                        if let Some(callback) = &mut self.announce {
                            callback(self.floors[self.current_floor_index].pronunciation);
                        }
                        self.set_door(DoorState::Opening(progress + 5)); // 2 secs to complete
                    }
                    _ => {
                        self.set_door(DoorState::Opening(progress + 5)); // 2 secs to complete
                    }
                }
            }
            DoorState::Open(progress) => {
                if progress == 100 {
                    self.set_door(DoorState::Closing(0));
                } else {
                    self.set_door(DoorState::Open(progress + 2)); // 5 secs to complete
                }
            }
            DoorState::Closing(progress) => {
                match progress {
                    100 => {
                        if self.floors[self.current_floor_index].stop {
                            let floor = &mut self.floors[self.current_floor_index];
                            floor.stop = false;
                            floor.button.turn_off().unwrap();
                        }
                        self.set_door(DoorState::Closed);
                    }
                    0 => {
                        if let Some(callback) = &mut self.announce {
                            callback(b"do'aga/shimarima'_su.\r");
                        }
                        self.set_door(DoorState::Closing(progress + 5)); // 2 secs to complete
                    }
                    _ => {
                        self.set_door(DoorState::Closing(progress + 5)); // 2 secs to complete
                    }
                }
            }
            DoorState::Closed => {
                match self.direction {
                    Direction::Up(value) => {
                        if let Some(progress) = value {
                            if progress == 100 {
                                let index = self.current_floor_index + 1;
                                self.set_current_floor_index(index);
                                if self.floors[index].stop {
                                    self.set_door(DoorState::Opening(0));
                                    self.set_direction(if index == self.floors.len() - 1 {
                                        if self.floors[..index]
                                            .iter()
                                            .position(|f| f.stop)
                                            .is_some()
                                        {
                                            Direction::Down(None)
                                        } else {
                                            Direction::Idle
                                        }
                                    } else {
                                        Direction::Up(None)
                                    })
                                } else {
                                    self.set_direction(Direction::Up(Some(0)));
                                }
                            } else {
                                self.set_direction(Direction::Up(Some(progress + 2)));
                                // 5 secs to complete
                            }
                        } else {
                            self.goto_next_floor();
                        }
                    }
                    Direction::Down(value) => {
                        if let Some(progress) = value {
                            if progress == 100 {
                                let index = self.current_floor_index - 1;
                                self.set_current_floor_index(index);
                                if self.floors[index].stop {
                                    self.set_door(DoorState::Opening(0));
                                    self.set_direction(if index == 0 {
                                        if self.floors[1..].iter().position(|f| f.stop).is_some() {
                                            Direction::Up(None)
                                        } else {
                                            Direction::Idle
                                        }
                                    } else {
                                        Direction::Down(None)
                                    })
                                } else {
                                    self.set_direction(Direction::Down(Some(0)));
                                }
                            } else {
                                self.set_direction(Direction::Down(Some(progress + 2)));
                                // 5 secs to complete
                            }
                        } else {
                            self.goto_next_floor();
                        }
                    }
                    Direction::Idle => {
                        self.goto_next_floor();
                    }
                }
            }
        }
    }

    pub fn on_repaint<F>(&mut self, callback: F)
    where
        F: FnMut() + 'static,
    {
        self.repaint = Some(Box::new(callback));
    }

    pub fn on_announce<F>(&mut self, callback: F)
    where
        F: FnMut(&[u8]) + 'static,
    {
        self.announce = Some(Box::new(callback));
    }

    pub fn floor_to_index(&self, floor: i8) -> usize {
        self.floors.iter().position(|f| f.number == floor).unwrap()
    }

    pub fn index_to_floor(&self, index: usize) -> i8 {
        self.floors[index].number
    }

    pub fn set_door_open(&mut self, value: bool) -> bool {
        if value {
            match self.door {
                DoorState::Opening(_) => false,
                DoorState::Open(_) => {
                    self.set_door(DoorState::Open(0));
                    true
                }
                DoorState::Closing(progress) => {
                    self.set_door(DoorState::Opening(100 - progress));
                    true
                }
                DoorState::Closed => {
                    if self.direction == Direction::Idle {
                        self.set_door(DoorState::Opening(0));
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            match self.door {
                DoorState::Open(_) => {
                    self.set_door(DoorState::Closing(0));
                    true
                }
                _ => false,
            }
        }
    }
}

impl embedded_graphics::Drawable for Elevator {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let door_openess = match self.door {
            DoorState::Opening(progress) => progress,
            DoorState::Open(_) => 100,
            DoorState::Closing(progress) => 100 - progress,
            DoorState::Closed => 0,
        };
        let margin = 20u32;
        let door_width = (100 - door_openess) as u32 * (128 - margin * 2) / 100;
        let door_style = PrimitiveStyleBuilder::new()
            .fill_color(BinaryColor::On)
            .build();
        Rectangle::new(
            Point::new(0, 0),
            embedded_graphics::geometry::Size::new(door_width + margin, 64),
        )
        .draw_styled(&door_style, target)?;
        Rectangle::new(
            Point::new(128 - margin as i32 - door_width as i32, 0),
            embedded_graphics::geometry::Size::new(door_width + margin, 64),
        )
        .draw_styled(&door_style, target)?;

        let background_style_highlighted = PrimitiveStyleBuilder::new()
            .fill_color(BinaryColor::Off)
            .build();

        let text_style_highlighted = MonoTextStyleBuilder::new()
            .font(&FONT_5X8)
            .text_color(BinaryColor::On)
            .build();
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_5X8)
            .text_color(BinaryColor::Off)
            .build();

        // 128x64
        for i in 0..self.floors.len() {
            let floor = &self.floors[i];
            let y = 56 - i as i32 * 8;
            let width = 5 * floor.label.len() as i32;
            if floor.stop {
                Rectangle::new(Point::new(128 - 14, y), Size::new(14, 8))
                    .draw_styled(&background_style_highlighted, target)?;
                Text::new(
                    floor.label,
                    Point::new(128 - width - 2, y + 6),
                    text_style_highlighted,
                )
                .draw(target)?;
            } else {
                Text::new(floor.label, Point::new(128 - width - 2, y + 6), text_style)
                    .draw(target)?;
            }
            if i == self.current_floor_index {
                let y = match self.direction {
                    Direction::Up(value) => {
                        if let Some(progress) = value {
                            y - progress as i32 * 8 / 100
                        } else {
                            y
                        }
                    }
                    Direction::Down(value) => {
                        if let Some(progress) = value {
                            y + progress as i32 * 8 / 100
                        } else {
                            y
                        }
                    }
                    Direction::Idle => y,
                };
                Triangle::new(
                    Point::new(128 - margin as i32 + 1, y + 2),
                    Point::new(128 - margin as i32 + 1, y + 6),
                    Point::new(128 - margin as i32 + 4, y + 4),
                )
                .draw_styled(&background_style_highlighted, target)?;
            }
        }

        match self.direction {
            Direction::Up(value) => {
                let dy = if let Some(progress) = value {
                    progress as i32 / 5 % 10
                } else {
                    0
                };
                let y = 10 - dy;
                let height = 13;
                let width = 7;
                Triangle::new(
                    Point::new(margin as i32 / 2 - 1, y),
                    Point::new(margin as i32 / 2 - 1 - width, y + height),
                    Point::new(margin as i32 / 2 - 1 + width, y + height),
                )
                .draw_styled(
                    &PrimitiveStyleBuilder::new()
                        .fill_color(BinaryColor::Off)
                        .build(),
                    target,
                )?;
            }
            Direction::Down(value) => {
                let dy = if let Some(progress) = value {
                    progress as i32 / 5 % 10
                } else {
                    0
                };
                let y = 41 + dy;
                let height = 13;
                let width = 7;
                Triangle::new(
                    Point::new(margin as i32 / 2 - 1, y + height),
                    Point::new(margin as i32 / 2 - 1 - width, y),
                    Point::new(margin as i32 / 2 - 1 + width, y),
                )
                .draw_styled(
                    &PrimitiveStyleBuilder::new()
                        .fill_color(BinaryColor::Off)
                        .build(),
                    target,
                )?;
            }
            Direction::Idle => {}
        }
        let label = self.floors[self.current_floor_index].label;
        Text::new(
            label,
            Point::new(10 - label.len() as i32 * 10 / 2, 38),
            MonoTextStyleBuilder::new()
                .font(&FONT_10X20)
                .text_color(BinaryColor::Off)
                .build(),
        )
        .draw(target)?;
        Ok(())
    }
}

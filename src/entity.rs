// 'entity.rs'

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Entity {
    pub type_id: u16,
    pub components: Vec<Box<dyn Component>>,
}

impl Entity {
    pub fn new(type_id: u16) -> Self {
        Self {
            type_id,
            components: vec![],
        }
    }

    pub fn with(mut self, component: Box<dyn Component>) -> Self {
        self.components.push(component);
        self
    }

    pub fn serialize_to_binary(&mut self) -> Vec<u8> {
        bincode::serialize(self).expect("Entity serialization error")
    }

    pub fn deserialize_from_binary(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).expect("Entity deserialization error")
    }
}

#[typetag::serde]
pub trait Component { }

#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentPosition {
    pub x: f64,
    pub y: f64,
}

#[typetag::serde]
impl Component for ComponentPosition { }

pub fn test_create_player_entity() {
    // Сборка сущности игрока
    let mut player = Entity::new(1)
        .with(Box::new(ComponentPosition { x: 500.25, y: -120.0 }));

    // Упаковываем в бинарник
    let binary_data: Vec<u8> = player.serialize_to_binary();

    // Выведем размер и сами байты, чтобы убедиться, насколько всё компактно
    println!("Размер бинарных данных: {} байт", binary_data.len());
    println!("Сырые байты: {:?}", binary_data);

    // Распаковываем обратно
    let loaded_player = Entity::deserialize_from_binary(&binary_data);
    println!("Сущность успешно загружена! Количество компонентов: {}", loaded_player.components.len());
}


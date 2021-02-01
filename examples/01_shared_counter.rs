use premer::{Conf, Premer, TickContext};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct GameState {
    counter: i64,
}

impl premer::GameState for GameState {
    type Event = Event;
    type Input = Input;

    fn tick(&mut self, context: &TickContext<Self::Input>, events: &mut Vec<Self::Event>) {
        let change_count = context
            .inputs()
            .filter(|i| i.changed())
            .inspect(|i| {
                dbg!(i);
            })
            .filter(|i| match i.now() {
                Input::Increment => {
                    self.counter += 1;
                    true
                }
                Input::Decrement => {
                    self.counter -= 1;
                    true
                }
                Input::None => false,
            })
            .count();
        // dbg!(change_count);
        if change_count > 0 {
            events.push(Event::ValueChanged);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Input {
    Increment,
    Decrement,
    None,
}

#[derive(Debug)]
enum Event {
    ValueChanged,
}

fn main() {
    use std::sync::mpsc::*;

    let (input_tx, input_rx) = channel();
    std::thread::spawn(move || {
        use device_query::*;

        let device_state = DeviceState::new();
        loop {
            let input = device_state
                .get_keys()
                .into_iter()
                .filter_map(|key| match key {
                    Keycode::W => Some(Input::Increment),
                    Keycode::S => Some(Input::Decrement),
                    _ => None,
                })
                .next()
                .unwrap_or(Input::None);

            input_tx.send((std::time::Instant::now(), input)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    let mut premer = Premer::new(GameState { counter: 0 });
    let player = premer.create_local_player();

    loop {
        input_rx
            .iter()
            .filter_map(|(instant, input)| {
                premer.set_input_state_instant(player, instant, input).ok()
            })
            .next();
        for event in premer.tick() {
            match event {
                Conf::Confirmed(Event::ValueChanged) => {
                    dbg!(premer.to_render());
                }
                e => {
                    dbg!(e);
                }
            }
        }
    }
}

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

pub trait GameState: Sized {
    type Event;
    type Input: PartialEq;

    fn tick(&mut self, context: &TickContext<Self::Input>, events: &mut Vec<Self::Event>);
}

type PlayerInputMap<I> = HashMap<PlayerId, InputHistory<I>>;

pub struct Premer<S: GameState> {
    state: S,
    state_as_of: Instant,
    player_input_states: PlayerInputMap<S::Input>,
}

impl<S: GameState> Premer<S> {
    pub fn new(initial_state: S) -> Self {
        Premer {
            state: initial_state,
            state_as_of: Instant::now(),
            player_input_states: HashMap::new(),
        }
    }

    pub fn create_local_player(&mut self) -> PlayerId {
        PlayerId(0)
    }

    pub fn set_input_state(&mut self, player: PlayerId, input: S::Input) {
        self.set_input_state_instant(player, Instant::now(), input)
            .unwrap();
    }

    pub fn set_input_state_instant(
        &mut self,
        player: PlayerId,
        instant: Instant,
        input: S::Input,
    ) -> Result<(), InputWouldBeDropped> {
        if instant <= self.state_as_of {
            return Err(InputWouldBeDropped);
        }
        self.player_input_states
            .entry(player)
            .or_insert_with(|| InputHistory::new())
            .insert(instant, input);
        Ok(())
    }

    pub fn tick(&mut self) -> impl Iterator<Item = Conf<S::Event>> {
        let mut conf_events = Vec::new();

        loop {
            let (last_loop, tick_to) = match self.smart_tick_to() {
                Some(i) => (false, i),
                None => (true, Instant::now()),
            };
            let tick_amount = tick_to - self.state_as_of;

            let context =
                TickContext::new(self.state_as_of, tick_amount, &self.player_input_states);

            let mut events = Vec::new();
            self.state.tick(&context, &mut events);
            conf_events.extend(events.into_iter().map(Conf::Confirmed));

            self.state_as_of += tick_amount;
            if last_loop {
                break;
            }
        }

        conf_events.into_iter()
    }

    fn smart_tick_to(&self) -> Option<Instant> {
        self.player_input_states
            .iter()
            .filter_map(|(_, history)| {
                history
                    .next_after(self.state_as_of)
                    .map(|(instant, _)| *instant)
            })
            .min()
    }

    pub fn to_render(&self) -> &S {
        &self.state
    }
}

#[derive(Debug)]
pub struct InputWouldBeDropped;

struct InputHistory<I> {
    map: BTreeMap<Instant, I>,
}

impl<I: PartialEq> InputHistory<I> {
    fn new() -> Self {
        InputHistory {
            map: BTreeMap::new(),
        }
    }

    fn insert(&mut self, instant: Instant, value: I) {
        if self.as_of(instant) == Some(&value) {
            return;
        }

        dbg!(Instant::now());
        dbg!(self.map.len());
        self.map.insert(instant, value);
    }

    fn as_of(&self, instant: Instant) -> Option<&I> {
        self.map.range(..=instant).next_back().map(|kv| kv.1)
    }

    fn next_after(&self, instant: Instant) -> Option<(&Instant, &I)> {
        use core::ops::Bound::*;

        self.map.range((Excluded(instant), Unbounded)).next()
    }
}

pub struct TickContext<'p, I> {
    player_input_states: &'p PlayerInputMap<I>,
    last_tick: Instant,
    tick_amount: Duration,
}

impl<'p, I> TickContext<'p, I> {
    fn new(last_tick: Instant, tick_amount: Duration, input_map: &'p PlayerInputMap<I>) -> Self {
        TickContext {
            player_input_states: input_map,
            last_tick,
            tick_amount,
        }
    }

    pub fn inputs(&self) -> impl Iterator<Item = PlayerInput<I>>
    where
        I: PartialEq,
    {
        self.player_input_states
            .iter()
            .filter_map(move |(player, history)| {
                let now = history.as_of(self.last_tick + self.tick_amount)?;
                let last = history.as_of(self.last_tick);
                Some(PlayerInput {
                    player: *player,
                    last,
                    now,
                })
            })
    }
}

#[derive(Debug)]
pub struct PlayerInput<'p, I> {
    player: PlayerId,
    last: Option<&'p I>,
    now: &'p I,
}

impl<'p, I> PlayerInput<'p, I> {
    pub fn changed(&self) -> bool
    where
        I: PartialEq,
    {
        self.last != Some(self.now)
    }

    pub fn now(&self) -> &I {
        &self.now
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(u32);

#[derive(Debug)]
pub enum Conf<T> {
    Predicted(T),
    Confirmed(T),
}

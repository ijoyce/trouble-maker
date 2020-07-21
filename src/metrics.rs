use std::fmt;

#[derive(Debug)]
pub struct Counter {
    name: String,
    value: u32,
}

impl Counter {
    pub fn new(name: String) -> Self {
        Counter { name, value: 0 }
    }
    pub fn increment(&mut self) -> u32 {
        self.value += 1;
        self.value
    }
}

#[derive(Debug)]
pub struct Metrics {
    pub requests: Counter,
}

impl Metrics {
    pub fn new() -> Metrics {
        return Metrics {
            requests: Counter::new(String::from("Requests")),
        };
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Request Count: {}", self.requests.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_counter_is_initialized_to_0() {
        let c = Counter::new(String::from("test"));
        assert_eq!(c.value, 0);
    }

    #[test]
    fn incrementing_a_counter_3_times_has_value_of_3() {
        let mut c = Counter::new(String::from("test"));
        c.increment();
        c.increment();
        c.increment();
        assert_eq!(c.value, 3);
    }
}

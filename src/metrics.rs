use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct Counter {
    name: String,
    pub value: u32,
}

impl Counter {
    pub fn new(name: String) -> Self {
        Counter { name, value: 0 }
    }
    pub fn increment(&mut self) -> u32 {
        self.value += 1;
        self.value
    }

    pub fn decrement(&mut self) -> u32 {
        self.value -= 1;
        self.value
    }
}

#[derive(Debug, Serialize)]
pub struct Metrics {
    pub requests: Counter,
    pub delays: Counter,
    pub errors: Counter,
    pub timeouts: Counter,
    pub concurrent_requests: Counter,
    pub shed_requests: Counter,
}

impl Metrics {
    pub fn new() -> Metrics {
        Metrics {
            requests: Counter::new(String::from("Requests")),
            delays: Counter::new(String::from("Delays")),
            errors: Counter::new(String::from("Errors")),
            timeouts: Counter::new(String::from("Timeouts")),
            concurrent_requests: Counter::new(String::from("ConcurrentRequests")),
            shed_requests: Counter::new(String::from("ShedRequests")),
        }
    }

    pub fn to_json(&mut self) -> String {
        serde_json::to_string(self).unwrap_or(String::from("Error generating metrics."))
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Request Count: {}.\nDelayed Requests: {}.\nErrored Requests: {}.\nTimedout Requets: {}.\n",
            self.requests.value, self.delays.value, self.errors.value, self.timeouts.value)
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

    #[test]
    fn new_metric_counters_are_init_to_0() {
        let m = Metrics::new();
        assert_eq!(m.requests.value, 0);
        assert_eq!(m.delays.value, 0);
        assert_eq!(m.errors.value, 0);
        assert_eq!(m.timeouts.value, 0);
        assert_eq!(m.concurrent_requests.value, 0);
        assert_eq!(m.shed_requests.value, 0);
    }

    #[test]
    fn incrementing_a_count_3_times_and_decrementing_2_times_has_a_value_of_1() {
        let mut c = Counter::new(String::from("test"));
        c.increment();
        c.increment();
        c.increment();
        c.decrement();
        c.decrement();
        assert_eq!(c.value, 1);
    }
}

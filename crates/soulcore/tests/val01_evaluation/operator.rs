//! Explicit, synthetic-only loopback operator. Never a product IPC surface.
use super::{
    manual,
    presentation::{PresentationError, SimulatedApproval, Simulation},
    synthetic_cards::CARDS,
};
use serde_json::{json, Value};
use std::{
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

#[path = "operator/feedback.rs"]
mod feedback;
#[path = "operator/http.rs"]
mod http;
#[path = "operator/tests.rs"]
mod tests;

use feedback::Action;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Welcome,
    Pair,
    Recorded,
    Revealed,
    Finished,
}

// No Debug/Serialize: the only prose projection is the active blind response.
struct Operator {
    simulation: Simulation,
    stage: Stage,
    card: usize,
    view: Option<Value>,
    presented_at: Duration,
    deadline: Duration,
    last_elapsed: Duration,
}

impl Operator {
    fn new(seconds: u64) -> Self {
        Self {
            simulation: Simulation::with_window(0, seconds),
            stage: Stage::Welcome,
            card: 0,
            view: None,
            presented_at: Duration::ZERO,
            deadline: Duration::from_secs(seconds),
            last_elapsed: Duration::ZERO,
        }
    }

    fn handle(&mut self, body: &[u8], elapsed: Duration) -> Result<Value, &'static str> {
        let action: Action = serde_json::from_slice(body).map_err(|_| "bad_request")?;
        if elapsed < self.last_elapsed {
            return Err("clock");
        }
        self.last_elapsed = elapsed;
        if elapsed >= self.deadline {
            self.finish(false);
        }
        match action {
            Action::State {} => self.state(),
            Action::Stop {} => {
                self.finish(false);
                self.state()
            }
            Action::Withdraw {} => {
                self.finish(true);
                self.state()
            }
            Action::Report {} if self.stage == Stage::Finished => self.state(),
            Action::Start {} if self.stage == Stage::Welcome => self.show_next(0, elapsed),
            Action::Submit { card, feedback } if self.stage == Stage::Pair && card == self.card => {
                let converted = feedback.convert(
                    CARDS[card].card().task,
                    elapsed.saturating_sub(self.presented_at),
                    self.deadline,
                )?;
                self.simulation
                    .submit_operator(CARDS[card], elapsed.as_secs(), converted)
                    .map_err(|_| "feedback")?;
                self.view = None;
                self.stage = Stage::Recorded;
                self.state()
            }
            Action::Reveal { card } if self.stage == Stage::Recorded && card == self.card => {
                let mapping = self.simulation.reveal(CARDS[card]).map_err(|_| "state")?;
                self.stage = Stage::Revealed;
                self.view = Some(json!({
                    "stage":"revealed","card":card,"label":CARDS[card].card().label,
                    "task":CARDS[card].card().task,"mapping":mapping
                }));
                self.state()
            }
            Action::Next { card } if self.stage == Stage::Revealed && card == self.card => {
                self.show_next(card + 1, elapsed)
            }
            _ => Err("state"),
        }
    }

    fn show_next(&mut self, start: usize, elapsed: Duration) -> Result<Value, &'static str> {
        self.view = None;
        let began = Instant::now();
        for (index, id) in CARDS.iter().copied().enumerate().skip(start) {
            let now = elapsed + began.elapsed();
            if now >= self.deadline {
                self.finish(false);
                return self.state();
            }
            match self
                .simulation
                .generate(id, now.as_secs(), SimulatedApproval::ApproveExact)
            {
                Ok(()) => {}
                Err(PresentationError::PathPaused) => continue,
                Err(_) => {
                    self.finish(false);
                    return Err("generation");
                }
            }
            let now = elapsed + began.elapsed();
            if now >= self.deadline {
                self.finish(false);
                return self.state();
            }
            let mut response = None;
            self.simulation
                .present(id, now.as_secs(), |pair| {
                    response = Some(json!({
                        "stage":"pair", "card":index, "label":pair.card,
                        "task":id.card().task, "instruction":pair.instruction,
                        "left_text":pair.left_text, "right_text":pair.right_text
                    }));
                })
                .map_err(|_| "generation")?;
            self.view = response;
            self.card = index;
            self.presented_at = now;
            self.stage = Stage::Pair;
            return self.state();
        }
        self.finish(false);
        self.state()
    }

    fn finish(&mut self, withdraw: bool) {
        self.view = None;
        if withdraw {
            self.simulation.withdraw();
        } else {
            self.simulation.stop();
        }
        self.stage = Stage::Finished;
    }

    fn state(&self) -> Result<Value, &'static str> {
        match self.stage {
            Stage::Welcome => Ok(json!({"stage":"welcome"})),
            Stage::Pair | Stage::Revealed => self.view.clone().ok_or("state"),
            Stage::Recorded => Ok(json!({
                "stage":"recorded","card":self.card,"label":CARDS[self.card].card().label,
                "task":CARDS[self.card].card().task
            })),
            Stage::Finished => {
                let metadata = self.simulation.export_records();
                let serialized = serde_json::to_string(&metadata).map_err(|_| "metadata")?;
                let summary = manual::report_json(&serialized).map_err(|_| "metadata")?;
                Ok(json!({"stage":"finished","report":{
                    "synthetic":true, "human_evaluation":false,
                    "feedback_origin":"operator_on_synthetic",
                    "metadata":metadata, "summary":summary
                }}))
            }
        }
    }
}

fn connection(
    stream: TcpStream,
    host: &str,
    token: &str,
    operator: &mut Operator,
    elapsed: Duration,
) {
    http::connection(stream, host, token, operator, elapsed);
}

#[test]
#[ignore = "explicit synthetic operator via SOUL_VAL01_OPERATOR_MODE=synthetic"]
fn serve() -> Result<(), &'static str> {
    if std::env::var("SOUL_VAL01_OPERATOR_MODE").as_deref() != Ok("synthetic") {
        return Err("synthetic_mode_required");
    }
    let seconds = match std::env::var("SOUL_VAL01_OPERATOR_SECONDS") {
        Ok(value) => value.parse::<u64>().map_err(|_| "invalid_seconds")?,
        Err(std::env::VarError::NotPresent) => 900,
        Err(_) => return Err("invalid_seconds"),
    };
    if !(1..=3600).contains(&seconds) {
        return Err("invalid_seconds");
    }
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "bind")?;
    listener.set_nonblocking(true).map_err(|_| "listener")?;
    let host = listener.local_addr().map_err(|_| "listener")?.to_string();
    let token = uuid::Uuid::now_v7().to_string();
    let mut operator = Operator::new(seconds);
    let started = Instant::now();
    println!("{}", json!({"url":format!("http://{host}/{token}/")}));
    while started.elapsed() < operator.deadline && operator.stage != Stage::Finished {
        match listener.accept() {
            Ok((stream, peer)) if peer.ip().is_loopback() => {
                connection(stream, &host, &token, &mut operator, started.elapsed());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return Err("accept"),
        }
    }
    operator.finish(false);
    let final_state = operator.state()?;
    println!("{}", final_state["report"]);
    Ok(())
}

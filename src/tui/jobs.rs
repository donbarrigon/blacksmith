//! Procesos lanzados desde la interfaz: corren en segundo plano y su salida
//! llega por un canal, en trozos (no por líneas) para ver también lo que
//! queda sin salto de línea, como `¿Continuar? [s/N]: `.

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use ratatui::text::Line;

use super::ansi;
use crate::migrate::Status;

/// Máximo de líneas guardadas por proceso.
const MAX_LINES: usize = 5000;

/// Mensajes de los hilos de fondo al bucle principal.
pub enum Msg {
    Output {
        job: usize,
        stream: Stream,
        bytes: Vec<u8>,
    },
    /// Estado pedido por un `list`.
    Status(Result<Status, String>),
    /// Estado para el autocompletado (IDs de migraciones y seeds).
    Catalog(Result<Status, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Out = 0,
    Err = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Exited(Option<i32>),
    Killed,
}

pub struct Job {
    pub id: usize,
    /// La línea escrita, sin `run `.
    pub title: String,
    pub state: JobState,
    pub lines: Vec<Line<'static>>,
    /// Texto sin salto de línea todavía, por stream.
    partial: [String; 2],
    /// Bytes de un carácter UTF-8 cortado entre dos trozos, por stream.
    pending_utf8: [Vec<u8>; 2],
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    pub scroll: usize,
}

impl Job {
    /// Lanza `cmd` con stdin/stdout/stderr conectados a la interfaz.
    pub fn spawn(id: usize, title: String, mut cmd: Command, tx: &Sender<Msg>) -> std::io::Result<Job> {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("CARGO_TERM_COLOR", "always");
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take();
        if let Some(out) = child.stdout.take() {
            reader(id, Stream::Out, out, tx.clone());
        }
        if let Some(err) = child.stderr.take() {
            reader(id, Stream::Err, err, tx.clone());
        }
        Ok(Job {
            id,
            title,
            state: JobState::Running,
            lines: Vec::new(),
            partial: Default::default(),
            pending_utf8: Default::default(),
            child: Some(child),
            stdin,
            scroll: 0,
        })
    }

    /// Un job sin proceso, ya terminado, con un contenido fijo (ej. un diff).
    pub fn finished(id: usize, title: String, bytes: &[u8]) -> Job {
        let mut job = Job {
            id,
            title,
            state: JobState::Exited(Some(0)),
            lines: Vec::new(),
            partial: Default::default(),
            pending_utf8: Default::default(),
            child: None,
            stdin: None,
            scroll: 0,
        };
        job.push_bytes(Stream::Out, bytes);
        job
    }

    /// Un job sin proceso (para tests).
    #[cfg(test)]
    pub fn fake(id: usize, title: &str) -> Job {
        Job {
            id,
            title: title.into(),
            state: JobState::Running,
            lines: Vec::new(),
            partial: Default::default(),
            pending_utf8: Default::default(),
            child: None,
            stdin: None,
            scroll: 0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == JobState::Running
    }

    pub fn push_bytes(&mut self, stream: Stream, bytes: &[u8]) {
        let s = stream as usize;
        let mut buf = std::mem::take(&mut self.pending_utf8[s]);
        buf.extend_from_slice(bytes);
        let text = match std::str::from_utf8(&buf) {
            Ok(t) => t.to_string(),
            Err(e) if e.error_len().is_none() => {
                // Carácter cortado al final: se guarda para el próximo trozo.
                let (ok, rest) = buf.split_at(e.valid_up_to());
                self.pending_utf8[s] = rest.to_vec();
                String::from_utf8_lossy(ok).into_owned()
            }
            Err(_) => String::from_utf8_lossy(&buf).into_owned(),
        };
        for (i, piece) in text.split('\n').enumerate() {
            if i > 0 {
                let done = std::mem::take(&mut self.partial[s]);
                self.push_line(ansi::to_line(&done));
            }
            self.partial[s].push_str(piece);
        }
    }

    pub fn push_line(&mut self, line: Line<'static>) {
        self.lines.push(line);
        if self.lines.len() > MAX_LINES {
            self.lines.drain(..self.lines.len() - MAX_LINES);
        }
    }

    /// Líneas para mostrar: las completas más lo que quedó sin salto.
    pub fn view(&self) -> impl Iterator<Item = Line<'static>> + '_ {
        let partial = self.partial.iter().filter(|p| !p.is_empty()).map(|p| ansi::to_line(p));
        self.lines.iter().cloned().chain(partial)
    }

    pub fn view_len(&self) -> usize {
        self.lines.len() + self.partial.iter().filter(|p| !p.is_empty()).count()
    }

    /// Corriendo y con una pregunta sin responder (texto en stdout sin salto).
    pub fn awaiting_input(&self) -> bool {
        self.is_running() && self.stdin.is_some() && !self.partial[Stream::Out as usize].trim().is_empty()
    }

    /// Manda una línea al stdin del proceso y la deja escrita tras la pregunta.
    pub fn send_line(&mut self, text: &str) -> std::io::Result<()> {
        if let Some(stdin) = &mut self.stdin {
            writeln!(stdin, "{text}")?;
            stdin.flush()?;
        }
        let question = std::mem::take(&mut self.partial[Stream::Out as usize]);
        self.push_line(ansi::to_line(&format!("{question}{text}")));
        Ok(())
    }

    /// Revisa si el proceso terminó. `true` si cambió de estado.
    pub fn poll_exit(&mut self) -> bool {
        if !self.is_running() {
            return false;
        }
        let Some(child) = &mut self.child else { return false };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.finish(status);
                true
            }
            Ok(None) => false,
            Err(_) => {
                self.state = JobState::Exited(None);
                true
            }
        }
    }

    fn finish(&mut self, status: ExitStatus) {
        if self.state == JobState::Running {
            self.state = JobState::Exited(status.code());
        }
        self.stdin = None;
    }

    pub fn kill(&mut self) {
        if !self.is_running() {
            return;
        }
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.state = JobState::Killed;
        self.stdin = None;
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.kill();
    }
}

fn reader(job: usize, stream: Stream, mut src: impl Read + Send + 'static, tx: Sender<Msg>) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match src.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let msg = Msg::Output {
                        job,
                        stream,
                        bytes: buf[..n].to_vec(),
                    };
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junta_trozos_y_detecta_pregunta() {
        let mut job = Job::fake(1, "x");
        job.push_bytes(Stream::Out, b"uno\ndo");
        job.push_bytes(Stream::Out, b"s\n");
        let acento = "migración\n".as_bytes();
        job.push_bytes(Stream::Out, &acento[..8]); // corta la ó a la mitad
        job.push_bytes(Stream::Out, &acento[8..]);
        let text: Vec<String> = job.view().map(|l| l.to_string()).collect();
        assert_eq!(text, ["uno", "dos", "migración"]);
        assert!(!job.awaiting_input()); // sin stdin (fake)

        job.push_bytes(Stream::Out, "¿Continuar? [s/N]: ".as_bytes());
        assert_eq!(job.view_len(), 4);
        job.send_line("s").unwrap();
        assert_eq!(job.lines.last().unwrap().to_string(), "¿Continuar? [s/N]: s");
        assert_eq!(job.view_len(), 4);
    }

    #[test]
    fn proceso_real_pregunta_y_termina() {
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let (tx, rx) = mpsc::channel();
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            "printf '¿Continuar? [s/N]: '; read a; echo \"respuesta=$a\"; echo error >&2; exit 3",
        ]);
        let mut job = Job::spawn(1, "sh".into(), cmd, &tx).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut answered = false;
        while job.is_running() || job.view_len() < 3 {
            assert!(Instant::now() < deadline, "el proceso no terminó");
            if let Ok(Msg::Output { stream, bytes, .. }) = rx.recv_timeout(Duration::from_millis(20)) {
                job.push_bytes(stream, &bytes);
            }
            if !answered && job.awaiting_input() {
                job.send_line("s").unwrap();
                answered = true;
            }
            job.poll_exit();
        }
        assert!(answered);
        assert_eq!(job.state, JobState::Exited(Some(3)));
        let text: Vec<String> = job.view().map(|l| l.to_string()).collect();
        assert!(text.contains(&"¿Continuar? [s/N]: s".to_string()), "{text:?}");
        assert!(text.contains(&"respuesta=s".to_string()), "{text:?}");
        assert!(text.contains(&"error".to_string()), "{text:?}");
    }
}

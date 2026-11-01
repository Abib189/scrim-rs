//! Only the client a session gave an id to can resume it (upstream FigCraft 10702311, audit #1054
//! COL-1). Before, a resume was authorised by the session id and the client id alone, both of
//! which every participant sees, so anyone holding the invite could take over anyone's id, the
//! host's included. Peers on either side of the change still read each other.

use serde::Deserialize;

use crate::message::{self, Access, CatchUp, ClientMsg, Hello, Peer, Processed, ServerMsg};
use crate::{Client, Limits, NoRecompute, Outgoing, Resume, Server};
use figcraft_doc::Document;

const MAX: usize = 64 << 20;

fn hello() -> Hello {
    let ClientMsg::Hello(h) = Client::hello("f", "tok", "Ada", "#FF8800") else { panic!("not a hello") };
    h
}

/// A build from before the change: the same hello, announcing nothing new.
fn old_hello() -> Hello {
    Hello { client_version: "0.7.0 signals hosting".into(), ..hello() }
}

fn welcome_of(out: Vec<Outgoing>) -> ServerMsg {
    match out.into_iter().next() {
        Some(Outgoing::To(_, w @ ServerMsg::Welcome { .. })) => w,
        _ => panic!("no welcome"),
    }
}

fn session_of(w: &ServerMsg) -> u64 {
    let ServerMsg::Welcome { session, .. } = w else { panic!() };
    *session
}

#[test]
fn a_participant_cannot_take_over_another_participants_id() {
    let mut s = Server::new(Document::new("x"), "tok", Limits::default());
    let (host, out) = s.join(hello()).unwrap();
    let session = session_of(&welcome_of(out));
    // Someone with the invite claims the host's id, with no secret or with a guessed one.
    for secret in [0, 1, 0xDEAD_BEEF] {
        let h = Hello { name: "Mallory".into(), resume: Some(Resume { session, client_id: host, seq: 0, secret }), ..hello() };
        if let Ok((id, _)) = s.join(h) {
            assert_ne!(id, host, "a resume with secret {secret:#x} took over client {host}");
        }
    }
    // An older build's resume (no secret at all) can't either.
    let h = Hello { resume: Some(Resume { session, client_id: host, seq: 0, secret: 0 }), ..old_hello() };
    if let Ok((id, _)) = s.join(h) {
        assert_ne!(id, host);
    }
}

#[test]
fn a_client_coming_back_with_its_own_secret_keeps_its_id() {
    let mut s = Server::new(Document::new("x"), "tok", Limits::default());
    let (id, out) = s.join(hello()).unwrap();
    let mut c = Client::join(welcome_of(out)).unwrap();
    assert_ne!(c.resume().secret, 0, "a client that announces it gets a secret");
    s.leave(id);
    let ClientMsg::Hello(h) = c.reconnect(hello()) else { panic!() };
    let (back, out) = s.join(h).unwrap();
    assert_eq!(back, id, "the client the id was given to gets it back");
    c.receive(welcome_of(out), &NoRecompute).unwrap();
}

/// The welcome and hello as builds from before the change wrote and read them.
#[derive(Deserialize)]
enum OldServerMsg {
    Welcome {
        #[allow(dead_code)]
        session: u64,
        client_id: u32,
        #[allow(dead_code)]
        seq: u64,
        #[allow(dead_code)]
        catch_up: CatchUp,
        #[allow(dead_code)]
        processed: Processed,
        #[allow(dead_code)]
        peers: Vec<Peer>,
        #[allow(dead_code)]
        access: Access,
    },
}

#[derive(Debug, PartialEq, serde::Serialize, Deserialize)]
struct OldResume {
    session: u64,
    client_id: u32,
    seq: u64,
}

#[derive(serde::Serialize)]
struct OldHello {
    protocol: u32,
    file: String,
    token: String,
    name: String,
    color: String,
    client_version: String,
    resume: Option<OldResume>,
}

#[derive(serde::Serialize)]
enum OldClientMsg {
    Hello(OldHello),
}

#[test]
fn an_older_client_reads_its_welcome_and_rejoins_without_taking_an_id() {
    let mut s = Server::new(Document::new("x"), "tok", Limits::default());
    let (id, out) = s.join(old_hello()).unwrap();
    let welcome = welcome_of(out);
    let session = session_of(&welcome);
    // Its welcome is the one it always read (no secret appended).
    let bytes = message::encode(&welcome).unwrap();
    let OldServerMsg::Welcome { client_id, .. } = message::decode::<OldServerMsg>(&bytes, MAX).unwrap();
    assert_eq!(client_id, id);
    // Its hello, resume and all, is read; the resume (which can't prove the id) makes it a new
    // participant, never someone else.
    s.leave(id);
    let old = OldClientMsg::Hello(OldHello {
        protocol: message::PROTOCOL,
        file: "f".into(),
        token: "tok".into(),
        name: "Ada".into(),
        color: "#FF8800".into(),
        client_version: "0.7.0 signals hosting".into(),
        resume: Some(OldResume { session, client_id: id, seq: 0 }),
    });
    let ClientMsg::Hello(h) = message::decode_peer::<ClientMsg>(&message::encode(&old).unwrap(), MAX).unwrap() else { panic!() };
    assert_eq!(h.resume.map(|r| r.secret), Some(0));
    let (again, _) = s.join(h).unwrap();
    assert_ne!(again, id, "an id without a secret is never resumed");
}

#[test]
fn a_new_client_reads_an_older_servers_welcome_and_resumes_as_it_did() {
    // An older server's welcome has no secret: read as 0, and the resume goes back without one,
    // in the shape that server reads.
    let mut s = Server::new(Document::new("x"), "tok", Limits::default());
    let (_, out) = s.join(old_hello()).unwrap();
    let old_welcome = message::encode(&welcome_of(out)).unwrap();
    let w = message::decode_peer::<ServerMsg>(&old_welcome, MAX).unwrap();
    let mut c = Client::join(w).unwrap();
    assert_eq!(c.resume().secret, 0);
    let back = message::encode(&c.reconnect(hello())).unwrap();
    #[derive(Deserialize)]
    enum OldIn {
        Hello(OldInHello),
    }
    #[derive(Deserialize)]
    struct OldInHello {
        #[allow(dead_code)]
        protocol: u32,
        #[allow(dead_code)]
        file: String,
        #[allow(dead_code)]
        token: String,
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        color: String,
        #[allow(dead_code)]
        client_version: String,
        resume: Option<OldResume>,
    }
    let OldIn::Hello(h) = message::decode::<OldIn>(&back, MAX).expect("an older server reads the resume");
    assert!(h.resume.is_some());
}

#[test]
fn clients_that_read_a_secret_are_told_apart_from_those_that_dont() {
    assert!(message::reads_resume_secret(&message::client_version()));
    assert!(!message::reads_resume_secret("0.7.0 signals hosting"));
    // FigCraft has read the secret since 0.11.0 (it announces no word for it).
    assert!(message::reads_resume_secret("0.11.0 signals hosting"));
    assert!(message::reads_resume_secret("0.13.1 signals hosting comments packed"));
    assert!(!message::reads_resume_secret("0.10.0 signals hosting"));
    assert!(!message::reads_resume_secret(""));
    assert!(!message::reads_resume_secret("junk"));
}

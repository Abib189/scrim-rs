//! Fix AI C-7: of two joins asked for one after the other, the file asked for last comes to the
//! front, whichever join the network finishes first. It came down to which welcome arrived last,
//! so `figcraft-collab-net`'s `two_files_are_hosted_and_joined_independently` failed now and then
//! on a busy machine. Here the order is forced through the scripted link.

use figcraft_collab::message::{ClientMsg, ServerMsg};
use figcraft_collab::{Client, Outgoing};
use figcraft_doc::Document;
use serde_json::json;

use crate::live::{Event, Role};
use crate::live_tests::{info, peer, push, serve, session};

/// The welcome a client gets from a server for a file named `name`, and its client id.
fn welcome(name: &str) -> (u32, ServerMsg) {
    let mut doc = Document::new(name);
    doc.repair();
    let (mut server, _, _) = serve(&doc);
    let ClientMsg::Hello(h) = Client::hello("f", "", "Ada", "") else { panic!("no hello") };
    let (me, outs) = server.join(h).unwrap();
    let w = outs.into_iter().find_map(|o| if let Outgoing::To(_, w @ ServerMsg::Welcome { .. }) = o { Some(w) } else { None }).unwrap();
    (me, w)
}

#[test]
fn the_file_asked_for_last_is_in_front_whichever_join_finishes_first() {
    let (mut s, script) = session();
    s.execute("collab.join", &json!({"link": "figcraft://join?host=a:1&file=a&token=t"})).unwrap();
    let first = script.lock().unwrap().session;
    s.execute("collab.join", &json!({"link": "figcraft://join?host=b:1&file=b&token=t"})).unwrap();
    let second = script.lock().unwrap().session;
    assert!(second > first);
    // The second join finishes first.
    let (me, w) = welcome("Second");
    script.lock().unwrap().session = second;
    push(&script, vec![Event::Welcome { info: info(Role::Guest, me), peers: vec![peer(1, "Hana")] }, Event::Sync(w)]);
    s.live_poll();
    assert_eq!(s.doc().unwrap().name, "Second");
    // Then the first: it gets its tab, behind.
    let (me, w) = welcome("First");
    script.lock().unwrap().session = first;
    push(&script, vec![Event::Welcome { info: info(Role::Guest, me), peers: vec![peer(1, "Hana")] }, Event::Sync(w)]);
    s.live_poll();
    assert_eq!(s.docs.len(), 3, "your own file and both joined");
    assert!(s.docs.iter().any(|d| d.doc.name == "First"));
    assert_eq!(s.doc().unwrap().name, "Second", "the file asked for last stays in front");
}

#[test]
fn a_join_on_its_own_comes_to_the_front() {
    let (mut s, script) = session();
    s.execute("collab.join", &json!({"link": "figcraft://join?host=a:1&file=a&token=t"})).unwrap();
    let (me, w) = welcome("Only");
    push(&script, vec![Event::Welcome { info: info(Role::Guest, me), peers: vec![peer(1, "Hana")] }, Event::Sync(w)]);
    s.live_poll();
    assert_eq!(s.doc().unwrap().name, "Only");
}

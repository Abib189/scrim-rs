//! Run as the main.rs of a scratch binary crate outside the repo, with path dependencies
//! `ours` = scrim-rs/crates/collab, `ours-doc` = scrim-rs/crates/doc, `theirs` and `theirs-doc` = the same
//! crates of upstream FigCraft at 53259cf5. The output is c3-wire.txt.
//! Fix AI C-3: the messages a lone participant's client exchanges (presence, ping, pong) read the
//! same in Scrim's collab crate and upstream FigCraft head's (53259cf5), both directions. C-3
//! changes when pings are sent, not what they are.
fn main() {
    let max = 1 << 20;
    // Scrim's client to an upstream host.
    for (what, m) in [
        ("ping", ours::message::ClientMsg::Ping),
        ("presence", ours::message::ClientMsg::Presence(ours::message::Presence { cursor: Some((ours_doc::NodeId(1), 10.0, 20.0)), ..Default::default() })),
    ] {
        let b = ours::message::encode(&m).unwrap();
        let back: theirs::message::ClientMsg = theirs::message::decode(&b, max).unwrap();
        println!("scrim -> upstream {what}: {}", matches!((&m, &back), (ours::message::ClientMsg::Ping, theirs::message::ClientMsg::Ping) | (ours::message::ClientMsg::Presence(_), theirs::message::ClientMsg::Presence(_))));
    }
    // Upstream's host answering Scrim's client, and the other way round.
    let b = theirs::message::encode(&theirs::message::ServerMsg::Pong).unwrap();
    let back: ours::message::ServerMsg = ours::message::decode(&b, max).unwrap();
    println!("upstream -> scrim pong: {}", matches!(back, ours::message::ServerMsg::Pong));
    let b = ours::message::encode(&ours::message::ServerMsg::Pong).unwrap();
    let back: theirs::message::ServerMsg = theirs::message::decode(&b, max).unwrap();
    println!("scrim -> upstream pong: {}", matches!(back, theirs::message::ServerMsg::Pong));
    let b = theirs::message::encode(&theirs::message::ClientMsg::Ping).unwrap();
    let back: ours::message::ClientMsg = ours::message::decode(&b, max).unwrap();
    println!("upstream -> scrim ping: {}", matches!(back, ours::message::ClientMsg::Ping));
}

//! What opening a Scrim file says to the person about what it repaired or left out, and when it
//! is a FigCraft board ([`super::board_file`]).

use serde_json::json;

use crate::Session;

/// Say what opening file `name` repaired or left out (a damaged image, say): not only in the
/// result. Numbers an earlier build saved as `null`, reset as it opened (Fix AI LW-2), are said in
/// the interface language when that is all there is to say.
pub(super) fn say(s: &mut Session, name: &str, loaded: &figcraft_format::Loaded) {
    super::board_file::say(s, name, loaded);
    let reset = (loaded.repaired_numbers > 0).then(|| figcraft_format::null_numbers::note(loaded.repaired_numbers));
    if reset.is_some() && loaded.issues.iter().all(|i| Some(i) == reset.as_ref()) {
        s.request_ui("notice.numbersReset", json!({"name": name, "count": loaded.repaired_numbers}));
        return;
    }
    s.message = match loaded.issues.as_slice() {
        [] => return,
        [one] => Some(format!("{name}: {one}")),
        more => Some(format!("{name}: {} problems were repaired or left out while opening it, e.g. {}", more.len(), more[0])),
    };
}

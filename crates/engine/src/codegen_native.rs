//! SwiftUI and Jetpack Compose code for a layer tree (split out of `codegen.rs`): stacks for
//! auto layout, absolute offsets otherwise.

use super::*;

// ------------------------------------------------------------------ SwiftUI

fn swift_color(c: Color) -> String {
    format!("Color(red: {}, green: {}, blue: {}, opacity: {})", num(f64::from(c.r)), num(f64::from(c.g)), num(f64::from(c.b)), num(f64::from(c.a)))
}

fn swift_weight(w: u16) -> &'static str {
    match w {
        0..=150 => ".thin",
        151..=250 => ".ultraLight",
        251..=350 => ".light",
        351..=450 => ".regular",
        451..=550 => ".medium",
        551..=650 => ".semibold",
        651..=750 => ".bold",
        751..=850 => ".heavy",
        _ => ".black",
    }
}

/// SwiftUI for a layer and its subtree at `depth`; `extra` modifiers go last (a free child's
/// offset). A layer a boolean parameter shows sits in an `if`.
pub(crate) fn swift(ctx: &Ctx, id: NodeId, depth: usize, out: &mut String, extra: &[String]) {
    let Some(n) = ctx.doc.node(id) else { return };
    if id != ctx.root
        && let Some(p) = ctx.param(n, "visible")
    {
        let pad = indent(depth);
        out.push_str(&format!("{pad}if {} {{\n", p.ident));
        swift_layer(ctx, n, depth + 1, out, extra);
        out.push_str(&format!("{pad}}}\n"));
        return;
    }
    swift_layer(ctx, n, depth, out, extra);
}

fn swift_layer(ctx: &Ctx, n: &Node, depth: usize, out: &mut String, extra: &[String]) {
    let doc = ctx.doc;
    let pad = indent(depth);
    if ctx.too_deep(depth) {
        out.push_str(&format!("{pad}// {} …\n", n.name));
        return;
    }
    // Nested instances are calls; an instance swap parameter is a view.
    let call = match ctx.param(n, "mainComponent").filter(|_| n.id != ctx.root) {
        Some(p) => Some(format!("{pad}{}\n", p.ident)),
        None => ctx.call(n, CodeLang::SwiftUi, depth),
    };
    if let Some(call) = call {
        out.push_str(&call);
        for m in extra {
            out.push_str(&format!("{pad}    {m}\n"));
        }
        return;
    }
    let mut mods: Vec<String> = Vec::new();
    match &n.kind {
        NodeKind::Text(t) => {
            let text = ctx.param(n, "characters").map_or_else(|| literal(&t.characters, false), |p| p.ident.clone());
            out.push_str(&format!("{pad}Text({text})\n"));
            mods.push(format!(
                ".font(.custom({}, size: {}).weight({}))",
                literal(&t.style.font_family, false),
                num(t.style.font_size),
                swift_weight(t.style.weight())
            ));
            if let Some(c) = first_fill(n) {
                mods.push(format!(".foregroundColor({})", swift_color(c)));
            }
        }
        NodeKind::Ellipse { .. } => {
            out.push_str(&format!("{pad}Ellipse()\n"));
            if let Some(c) = first_fill(n) {
                mods.push(format!(".fill({})", swift_color(c)));
            }
            mods.push(format!(".frame(width: {}, height: {})", num(n.width), num(n.height)));
        }
        NodeKind::Rectangle => {
            out.push_str(&format!("{pad}RoundedRectangle(cornerRadius: {})\n", num(radius(n))));
            if let Some(c) = first_fill(n) {
                mods.push(format!(".fill({})", swift_color(c)));
            }
            mods.push(format!(".frame(width: {}, height: {})", num(n.width), num(n.height)));
        }
        k if k.is_frame_like() || matches!(k, NodeKind::Group) => {
            let children = ctx.kids(n);
            let open = match n.layout.as_ref().map(|l| (l.mode, l)) {
                Some((LayoutMode::Vertical, l)) => format!("VStack(alignment: .leading, spacing: {}) {{", num(l.gap)),
                Some((_, l)) => format!("HStack(alignment: .center, spacing: {}) {{", num(l.gap)),
                None => "ZStack(alignment: .topLeading) {".to_string(),
            };
            out.push_str(&format!("{pad}{open}\n"));
            // A slot shows the instance's content, or else the main's.
            let slot = ctx.param(n, "slot");
            let inner = match slot {
                Some(p) => {
                    out.push_str(&format!("{pad}    if let {0} {{\n{pad}        {0}\n{pad}    }} else {{\n", p.ident));
                    depth + 2
                }
                None => depth + 1,
            };
            for c in &children {
                // Free children sit at their offset inside a ZStack.
                let offset: Vec<String> = match doc.node(*c).filter(|_| n.layout.is_none()) {
                    Some(cn) => {
                        let [_, _, _, _, x, y] = cn.transform.as_coeffs();
                        vec![format!(".offset(x: {}, y: {})", num(x), num(y))]
                    }
                    None => Vec::new(),
                };
                swift(ctx, *c, inner, out, &offset);
            }
            if slot.is_some() {
                out.push_str(&format!("{pad}    }}\n"));
            }
            out.push_str(&format!("{pad}}}\n"));
            if let Some(l) = &n.layout {
                let [t, r, b, lft] = l.padding;
                if [t, r, b, lft].iter().any(|v| *v > 0.0) {
                    mods.push(format!(".padding(EdgeInsets(top: {}, leading: {}, bottom: {}, trailing: {}))", num(t), num(lft), num(b), num(r)));
                }
            }
            mods.push(format!(".frame(width: {}, height: {}, alignment: .topLeading)", num(n.width), num(n.height)));
            if let Some(c) = first_fill(n) {
                mods.push(format!(".background({})", swift_color(c)));
            }
            if radius(n) > 0.0 {
                mods.push(format!(".clipShape(RoundedRectangle(cornerRadius: {}))", num(radius(n))));
            }
        }
        _ => {
            out.push_str(&format!("{pad}// {}: export as an image asset\n{pad}Image({})\n", n.kind.label(), literal(&n.name, false)));
            mods.push(".resizable()".into());
            mods.push(format!(".frame(width: {}, height: {})", num(n.width), num(n.height)));
        }
    }
    if let Some(c) = stroke_color(n) {
        mods.push(format!(
            ".overlay(RoundedRectangle(cornerRadius: {}).stroke({}, lineWidth: {}))",
            num(radius(n)),
            swift_color(c),
            num(n.stroke.weight)
        ));
    }
    if let Some(s) = drop_shadow(n) {
        mods.push(format!(
            ".shadow(color: {}, radius: {}, x: {}, y: {})",
            swift_color(s.color),
            num(s.radius / 2.0),
            num(s.offset.0),
            num(s.offset.1)
        ));
    }
    if n.opacity < 1.0 {
        mods.push(format!(".opacity({})", num(f64::from(n.opacity))));
    }
    mods.extend(extra.iter().cloned());
    for m in mods {
        out.push_str(&format!("{pad}    {m}\n"));
    }
}

// ------------------------------------------------------------------ Jetpack Compose

fn kotlin_color(c: Color) -> String {
    let [r, g, b, a] = c.to_rgba8();
    format!("Color(0x{a:02X}{r:02X}{g:02X}{b:02X})")
}

/// The Modifier chain for a layer (size, shadow, background, border, alpha, padding).
fn modifier(n: &Node, shape: &str, padding: bool) -> String {
    let mut m = format!("Modifier.size({}.dp, {}.dp)", num(n.width), num(n.height));
    if let Some(s) = drop_shadow(n) {
        m.push_str(&format!(".shadow({}.dp, {shape})", num(s.radius / 2.0)));
    }
    if let Some(c) = first_fill(n) {
        m.push_str(&format!(".background({}, {shape})", kotlin_color(c)));
    }
    if let Some(c) = stroke_color(n) {
        m.push_str(&format!(".border({}.dp, {}, {shape})", num(n.stroke.weight), kotlin_color(c)));
    }
    if n.opacity < 1.0 {
        m.push_str(&format!(".alpha({}f)", num(f64::from(n.opacity))));
    }
    if padding && let Some(l) = &n.layout {
        let [t, r, b, lft] = l.padding;
        if [t, r, b, lft].iter().any(|v| *v > 0.0) {
            m.push_str(&format!(".padding(start = {}.dp, top = {}.dp, end = {}.dp, bottom = {}.dp)", num(lft), num(t), num(r), num(b)));
        }
    }
    m
}

/// Jetpack Compose for a layer and its subtree at `depth`. A layer a boolean parameter shows sits
/// in an `if`.
pub(crate) fn compose(ctx: &Ctx, id: NodeId, depth: usize, out: &mut String) {
    let Some(n) = ctx.doc.node(id) else { return };
    if id != ctx.root
        && let Some(p) = ctx.param(n, "visible")
    {
        let pad = indent(depth);
        out.push_str(&format!("{pad}if ({}) {{\n", p.ident));
        compose_layer(ctx, n, depth + 1, out);
        out.push_str(&format!("{pad}}}\n"));
        return;
    }
    compose_layer(ctx, n, depth, out);
}

fn compose_layer(ctx: &Ctx, n: &Node, depth: usize, out: &mut String) {
    let doc = ctx.doc;
    let pad = indent(depth);
    if ctx.too_deep(depth) {
        out.push_str(&format!("{pad}// {} …\n", n.name));
        return;
    }
    // Nested instances are calls; an instance swap parameter is a composable.
    let call = match ctx.param(n, "mainComponent").filter(|_| n.id != ctx.root) {
        Some(p) => Some(format!("{pad}{}()\n", p.ident)),
        None => ctx.call(n, CodeLang::Compose, depth),
    };
    if let Some(call) = call {
        out.push_str(&call);
        return;
    }
    let shape = if radius(n) > 0.0 { format!("RoundedCornerShape({}.dp)", num(radius(n))) } else { "RectangleShape".to_string() };
    match &n.kind {
        NodeKind::Text(t) => {
            let color = first_fill(n).map(kotlin_color).unwrap_or_else(|| "Color.Unspecified".into());
            let text = ctx.param(n, "characters").map_or_else(|| literal(&t.characters, true), |p| p.ident.clone());
            out.push_str(&format!(
                "{pad}Text(\n{pad}    text = {text},\n{pad}    fontSize = {}.sp,\n{pad}    fontWeight = FontWeight.W{},\n{pad}    color = {color},\n{pad})\n",
                num(t.style.font_size),
                (t.style.weight().clamp(100, 900) / 100) * 100
            ));
        }
        NodeKind::Ellipse { .. } => out.push_str(&format!("{pad}Box(modifier = {})\n", modifier(n, "CircleShape", false))),
        NodeKind::Rectangle => out.push_str(&format!("{pad}Box(modifier = {})\n", modifier(n, &shape, false))),
        k if k.is_frame_like() || matches!(k, NodeKind::Group) => {
            let children = ctx.kids(n);
            let open = match n.layout.as_ref().map(|l| (l.mode, l)) {
                Some((LayoutMode::Vertical, l)) => format!(
                    "Column(\n{pad}    modifier = {},\n{pad}    verticalArrangement = Arrangement.spacedBy({}.dp),\n{pad}) {{",
                    modifier(n, &shape, true),
                    num(l.gap)
                ),
                Some((_, l)) => format!(
                    "Row(\n{pad}    modifier = {},\n{pad}    horizontalArrangement = Arrangement.spacedBy({}.dp),\n{pad}    verticalAlignment = Alignment.CenterVertically,\n{pad}) {{",
                    modifier(n, &shape, true),
                    num(l.gap)
                ),
                None => format!("Box(modifier = {}) {{", modifier(n, &shape, false)),
            };
            out.push_str(&format!("{pad}{open}\n"));
            // A slot shows the instance's content, or else the main's.
            let slot = ctx.param(n, "slot");
            let base = match slot {
                Some(p) => {
                    out.push_str(&format!("{pad}    if ({0} != null) {{\n{pad}        {0}()\n{pad}    }} else {{\n", p.ident));
                    depth + 1
                }
                None => depth,
            };
            for c in &children {
                if n.layout.is_none()
                    && let Some(cn) = doc.node(*c)
                {
                    let [_, _, _, _, x, y] = cn.transform.as_coeffs();
                    out.push_str(&format!("{}Box(modifier = Modifier.offset({}.dp, {}.dp)) {{\n", indent(base + 1), num(x), num(y)));
                    compose(ctx, *c, base + 2, out);
                    out.push_str(&format!("{}}}\n", indent(base + 1)));
                } else {
                    compose(ctx, *c, base + 1, out);
                }
            }
            if slot.is_some() {
                out.push_str(&format!("{pad}    }}\n"));
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        _ => out.push_str(&format!(
            "{pad}// {}: export as an image asset\n{pad}Image(painter = painterResource(R.drawable.asset), contentDescription = {}, modifier = Modifier.size({}.dp, {}.dp))\n",
            n.kind.label(),
            literal(&n.name, true),
            num(n.width),
            num(n.height)
        )),
    }
}

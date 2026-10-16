use crate::{DomainIdentity, Value};
use std::{cell::RefCell, collections::{hash_map::Entry, HashMap}, path::PathBuf, rc::Rc};

/// Dropping a deeply nested `Environment` chain (thousands of `let`/currying
/// levels) would otherwise recurse through `Rc<RefCell<Frame>>`'s default
/// `Drop` one stack frame per level and could overflow the stack. `Environment`
/// has its own `Drop` below (mirroring `Value::Pair`'s iterative Drop in
/// `value.rs`) that walks the parent chain with an explicit worklist instead,
/// so this is fixed, not just documented as a live risk.
/// Session-wide print transcript plus a consumer cursor, so hot hosts
/// (REPL, LSP, swarm TCP) can take only the lines appended since their
/// last read instead of re-cloning the whole history per evaluation.
#[derive(Debug)]
pub struct Transcript {
    lines: Vec<String>,
    taken: usize,
}

#[derive(Clone, Debug)]
pub struct Environment(
    Rc<RefCell<Frame>>,
    Rc<RefCell<Transcript>>,
    Rc<RefCell<Limits>>,
);

#[derive(Debug)]
struct Frame {
    /// Кадр виклику замикання: параметри лежать у масиві `slots` за номером,
    /// імена — у `slot_names` (той самий порядок), тож пошук за іменем теж
    /// їх бачить. `None` — звичайний кадр без слотів (корінь, `child`).
    slot_names: Option<Rc<[Rc<str>]>>,
    slots: Vec<Value>,
    /// Тіло цього виклику не може додати нове ім'я в кадр (без `def`, `eval`,
    /// макросів і можливостей хоста) — розв'язувач може дивитися крізь нього.
    pure: bool,
    values: HashMap<Rc<str>, Value>,
    parent: Option<Environment>,
}

impl Frame {
    fn empty(parent: Option<Environment>) -> Self {
        Frame {
            slot_names: None,
            slots: Vec::new(),
            pure: false,
            values: HashMap::new(),
            parent,
        }
    }

    fn slot_index(&self, name: &str) -> Option<usize> {
        self.slot_names.as_ref()?.iter().position(|slot| **slot == *name)
    }
}

/// Opt-in resource/capability limits for one session, shared across every
/// lexical child. `None` remains the trusted native profile: unrestricted for
/// that dimension once the host capability layer is installed. Embeddings can
/// opt into narrower policies without changing language semantics.
/// Mechanically selected language Core profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreProfile {
    Core1,
    Core2,
    Core3,
    Core4,
}

#[derive(Debug, Default)]
struct Limits {
    selected_core_profile: Option<CoreProfile>,
    cons_limit: Option<usize>,
    cons_count: usize,
    numeric_bit_limit: Option<usize>,
    /// Exact program-name policy. `None` = unrestricted, `Some([])` = deny all.
    process_allowlist: Option<Vec<String>>,
    /// Filesystem roots are stored as caller-supplied paths. The host layer,
    /// not the language core, owns canonicalization/symlink enforcement.
    fs_read_roots: Option<Vec<PathBuf>>,
    fs_write_roots: Option<Vec<PathBuf>>,
    /// (host-or-bind-address, first-port, last-port), inclusive.
    tcp_connect_allowlist: Option<Vec<(String, u16, u16)>>,
    tcp_listen_allowlist: Option<Vec<(String, u16, u16)>>,
    /// Canonical language-owned bindings keyed by exact semantic domain.
    /// Equal payloads in different D1-D9 domains remain distinct HashMap keys.
    /// Residency alone never inserts a mechanism slot.
    domain_code_slots: HashMap<DomainIdentity, Value>,
    /// Historical exact-eight compatibility slots. These are intentionally
    /// separate storage so a legacy byte can never collide with a canonical
    /// domain identity merely from its packed payload.
    legacy_code_slots: HashMap<crate::CoreDomainIdentity, Value>,
}

impl Environment {
    pub fn root() -> Self {
        Self(
            Rc::new(RefCell::new(Frame::empty(None))),
            Rc::new(RefCell::new(Transcript {
                lines: Vec::new(),
                taken: 0,
            })),
            Rc::new(RefCell::new(Limits::default())),
        )
    }

    /// Opts this session into a maximum `cons` allocation count — past it,
    /// `cons` returns `ErrorKind::OutOfMemory` instead of succeeding.
    pub fn with_cons_limit(self, limit: usize) -> Self {
        self.2.borrow_mut().cons_limit = Some(limit);
        self
    }

    /// Opts this session into a maximum bit width for exact arithmetic
    /// results — past it arithmetic returns `ErrorKind::NumericOverflow`.
    pub fn with_numeric_bit_limit(self, limit: usize) -> Self {
        self.2.borrow_mut().numeric_bit_limit = Some(limit);
        self
    }

    /// Restricts process execution to exact program names.
    pub fn with_process_allowlist(self, programs: Vec<String>) -> Self {
        self.2.borrow_mut().process_allowlist = Some(programs);
        self
    }

    /// Restricts host filesystem reads (`read-file`, `read-file-bytes`,
    /// `read-dir`, `load`) to paths under one of these roots. Canonicalization
    /// and symlink checks are deliberately performed by `sens-host`.
    pub fn with_fs_read_roots(self, roots: Vec<PathBuf>) -> Self {
        self.2.borrow_mut().fs_read_roots = Some(roots);
        self
    }

    /// Restricts host filesystem writes (`write-file`, `write-file-bytes`) to
    /// paths under one of these roots.
    pub fn with_fs_write_roots(self, roots: Vec<PathBuf>) -> Self {
        self.2.borrow_mut().fs_write_roots = Some(roots);
        self
    }

    /// Restricts outbound TCP connects to explicit host + inclusive port
    /// ranges. `None` remains unrestricted; an empty list is deny-all.
    pub fn with_tcp_connect_allowlist(self, entries: Vec<(String, u16, u16)>) -> Self {
        self.2.borrow_mut().tcp_connect_allowlist = Some(entries);
        self
    }

    /// Restricts TCP listen/bind operations independently from connect.
    pub fn with_tcp_listen_allowlist(self, entries: Vec<(String, u16, u16)>) -> Self {
        self.2.borrow_mut().tcp_listen_allowlist = Some(entries);
        self
    }

    /// Host-owned canonicalization needs a snapshot of the configured roots;
    /// exposing data does not give the core any filesystem behavior itself.
    pub fn fs_read_roots(&self) -> Option<Vec<PathBuf>> {
        self.2.borrow().fs_read_roots.clone()
    }

    pub fn fs_write_roots(&self) -> Option<Vec<PathBuf>> {
        self.2.borrow().fs_write_roots.clone()
    }

    pub fn is_tcp_connect_allowed(&self, host: &str, port: u16) -> bool {
        match &self.2.borrow().tcp_connect_allowlist {
            Some(entries) => entries.iter().any(|(allowed_host, first, last)| {
                allowed_host == host && *first <= port && port <= *last
            }),
            None => true,
        }
    }

    pub fn is_tcp_listen_allowed(&self, address: &str, port: u16) -> bool {
        match &self.2.borrow().tcp_listen_allowlist {
            Some(entries) => entries.iter().any(|(allowed_address, first, last)| {
                allowed_address == address && *first <= port && port <= *last
            }),
            None => true,
        }
    }

    /// Called by `cons` before allocating; `Err(())` means the configured
    /// limit (if any) is already reached. No-op when unbounded.
    pub(crate) fn try_alloc_cons(&self) -> Result<(), ()> {
        let mut limits = self.2.borrow_mut();
        if let Some(limit) = limits.cons_limit {
            if limits.cons_count >= limit {
                return Err(());
            }
        }
        limits.cons_count += 1;
        Ok(())
    }

    pub(crate) fn numeric_bit_limit(&self) -> Option<usize> {
        self.2.borrow().numeric_bit_limit
    }

    /// Return the explicitly selected Core profile, if a profile loader selected one.
    pub fn selected_core_profile(&self) -> Option<CoreProfile> {
        self.2.borrow().selected_core_profile
    }

    /// Mechanism-only selector. Core meaning remains owned by SENS contracts.
    pub(crate) fn select_core_profile(&self, profile: CoreProfile) {
        self.2.borrow_mut().selected_core_profile = Some(profile);
    }

    /// Native root sessions are unrestricted (`None`). An embedding can set
    /// an exact allowlist; an empty allowlist is an explicit deny-all policy.
    pub fn is_process_allowed(&self, program: &str) -> bool {
        match &self.2.borrow().process_allowlist {
            Some(programs) => programs.iter().any(|allowed| allowed == program),
            None => true,
        }
    }

    /// Чи це кадр верхнього рівня сесії (без lexical parent).
    pub(crate) fn is_root(&self) -> bool {
        self.0.borrow().parent.is_none()
    }

    /// Canonical exact-domain language slot.
    pub(crate) fn domain_code_slot(
        &self,
        identity: impl Into<DomainIdentity>,
    ) -> Option<Value> {
        let identity = identity.into();
        self.2.borrow().domain_code_slots.get(&identity).cloned()
    }

    /// Binds one concrete mechanism to one already-known exact-domain identity.
    /// Width/residency alone never grants callability.
    pub(crate) fn bind_domain_code_slot_once(
        &self,
        identity: impl Into<DomainIdentity>,
        value: Value,
    ) -> bool {
        let identity = identity.into();
        let mut limits = self.2.borrow_mut();
        // Exact domain identity (width AND payload) is the whole key.
        // Vacant/Occupied prevents a second lookup and forbids overwrites.
        match limits.domain_code_slots.entry(identity) {
            Entry::Vacant(slot) => {
                slot.insert(value);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Transitional historical exact-eight compatibility lookup.
    ///
    /// Kept under the old method name so this slice does not grow CoreDomainIdentity API
    /// surface. New canonical code uses domain_code_slot.
    pub(crate) fn code_slot(&self, sid: crate::CoreDomainIdentity) -> Option<Value> {
        self.2
            .borrow()
            .legacy_code_slots
            .get(&sid)
            .cloned()
    }

    /// Transitional historical exact-eight compatibility binding.
    ///
    /// New canonical code uses bind_domain_code_slot_once.
    pub(crate) fn bind_code_slot_once(&self, sid: crate::CoreDomainIdentity, value: Value) -> bool {
        let mut limits = self.2.borrow_mut();
        // Historical W8 never shares a keyspace with exact D8.
        match limits.legacy_code_slots.entry(sid) {
            Entry::Vacant(slot) => {
                slot.insert(value);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// A child frame is the future lexical boundary captured by a closure. It
    /// shares transcript and all session policy/limits with its parent.
    pub fn child(&self) -> Self {
        Self(
            Rc::new(RefCell::new(Frame::empty(Some(self.clone())))),
            self.1.clone(),
            self.2.clone(),
        )
    }

    /// Кадр виклику замикання: значення параметрів за номером слота.
    pub(crate) fn child_with_slots(&self, names: Rc<[Rc<str>]>, slots: Vec<Value>, pure: bool) -> Self {
        let mut frame = Frame::empty(Some(self.clone()));
        frame.slot_names = Some(names);
        frame.slots = slots;
        frame.pure = pure;
        Self(Rc::new(RefCell::new(frame)), self.1.clone(), self.2.clone())
    }

    /// Значення `Local(depth, index)`: слот `index` кадру на `depth` кроків вище.
    pub(crate) fn get_local(&self, depth: u32, index: u32) -> Option<Value> {
        let mut frame = Rc::clone(&self.0);
        for _ in 0..depth {
            let parent = frame.borrow().parent.as_ref().map(|parent| Rc::clone(&parent.0))?;
            frame = parent;
        }
        let current = frame.borrow();
        current.slots.get(index as usize).cloned()
    }

    /// Імена параметрів кадрів, крізь які розв'язувач може бачити: від цього
    /// кадру вгору, доки кадри мають слоти; нечистий кадр — останній (його
    /// тіло може визначити нове ім'я, що затінить глибші).
    pub(crate) fn lexical_slot_scopes(&self) -> Vec<Rc<[Rc<str>]>> {
        let mut scopes = Vec::new();
        let mut frame = Rc::clone(&self.0);
        loop {
            let parent = {
                let current = frame.borrow();
                let Some(names) = &current.slot_names else { break };
                scopes.push(Rc::clone(names));
                if !current.pure {
                    break;
                }
                current.parent.as_ref().map(|parent| Rc::clone(&parent.0))
            };
            match parent {
                Some(parent) => frame = parent,
                None => break,
            }
        }
        scopes
    }

    /// Перепід'єднує лише безпосереднього lexical parent цього frame.
    /// Це host/UI-механізм для стабільного user-frame поверх змінної
    /// програмної поверхні; Lisp-семантику він не розширює.
    /// Відхиляє цикли та parent з іншими transcript/limits.
    pub fn reparent(&self, parent: Environment) -> Result<(), &'static str> {
        if !Rc::ptr_eq(&self.1, &parent.1) || !Rc::ptr_eq(&self.2, &parent.2) {
            return Err("new parent must share transcript and limits");
        }

        let mut current = Some(parent.clone());
        while let Some(environment) = current {
            if Rc::ptr_eq(&self.0, &environment.0) {
                return Err("reparent would create an environment cycle");
            }
            current = environment.0.borrow().parent.clone();
        }

        self.0.borrow_mut().parent = Some(parent);
        Ok(())
    }

    pub fn print(&self, line: String) {
        self.1.borrow_mut().lines.push(line);
    }

    pub fn output_snapshot(&self) -> Vec<String> {
        self.1.borrow().lines.clone()
    }

    pub fn output_take_new(&self) -> Vec<String> {
        let mut transcript = self.1.borrow_mut();
        let new = transcript.lines[transcript.taken..].to_vec();
        transcript.taken = transcript.lines.len();
        new
    }

    /// Snapshot of every visible binding, root-first, shadowed names resolved
    /// to their innermost value.
    pub fn snapshot(&self) -> Vec<(Rc<str>, Value)> {
        let mut frames = Vec::new();
        let mut current = Some(self.clone());
        while let Some(env) = current {
            frames.push(env.0.clone());
            current = env.0.borrow().parent.clone();
        }
        let mut out: Vec<(Rc<str>, Value)> = Vec::new();
        for frame in frames.iter().rev() {
            let f = frame.borrow();
            let slots = f.slot_names.iter().flat_map(|names| names.iter().zip(f.slots.iter()));
            for (name, value) in slots.chain(f.values.iter()) {
                match out.iter_mut().find(|(n, _)| n == name) {
                    Some(slot) => slot.1 = value.clone(),
                    None => out.push((name.clone(), value.clone())),
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub fn define(&self, name: impl Into<Rc<str>>, value: Value) {
        let name = name.into();
        let mut frame = self.0.borrow_mut();
        match frame.slot_index(&name) {
            Some(index) => frame.slots[index] = value,
            None => {
                frame.values.insert(name, value);
            }
        }
    }

    /// Updates the nearest already-existing lexical binding and never creates one.
    ///
    /// This is the storage mechanism used by the exact D5 SET/SETQ residents.
    /// Semantic authority remains with the D5 domain law; this method only mutates
    /// the selected environment cell after the evaluator has resolved the target.
    pub(crate) fn update_nearest_existing(&self, name: &str, value: Value) -> bool {
        let mut frame = Rc::clone(&self.0);
        loop {
            let parent = {
                let mut current = frame.borrow_mut();
                if let Some(index) = current.slot_index(name) {
                    current.slots[index] = value;
                    return true;
                }
                if let Some(slot) = current.values.get_mut(name) {
                    *slot = value;
                    return true;
                }
                current.parent.as_ref().map(|parent| Rc::clone(&parent.0))
            };
            let Some(next) = parent else {
                return false;
            };
            frame = next;
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        // Walk the frames themselves: cloning an `Environment` per step would
        // run its `Drop` for every step of every lookup.
        let mut frame = Rc::clone(&self.0);
        loop {
            let parent = {
                let current = frame.borrow();
                if let Some(index) = current.slot_index(name) {
                    return Some(current.slots[index].clone());
                }
                if let Some(value) = current.values.get(name) {
                    return Some(value.clone());
                }
                current.parent.as_ref().map(|parent| Rc::clone(&parent.0))
            };
            frame = parent?;
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        // Only the last owner of a frame frees it. Its parent chain is then
        // released iteratively: each frame that dies gives up its parent
        // before it drops, so no drop recurses through the chain (a deep
        // recursion would overflow the stack). Dropping a shared handle only
        // decrements a reference count and allocates nothing.
        if Rc::strong_count(&self.0) != 1 {
            return;
        }
        let Ok(mut frame) = self.0.try_borrow_mut() else {
            return;
        };
        let mut next = frame.parent.take();
        drop(frame);
        while let Some(environment) = next {
            next = if Rc::strong_count(&environment.0) == 1 {
                environment
                    .0
                    .try_borrow_mut()
                    .ok()
                    .and_then(|mut parent_frame| parent_frame.parent.take())
            } else {
                None
            };
            // `environment` now has no parent, so its own drop is shallow.
            drop(environment);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    pub environment: Environment,
}

impl Session {
    /// Construct only the mechanical root environment for programs made from
    /// already-admitted exact domain words. No Lisp macro definitions or Core4
    /// library are installed. Opt-in: `Session::default()` retains its
    /// historical macro-loaded embedding contract.
    ///
    /// Use `load_macro_library(&mut session)` before any program requiring
    /// macros, or `load_core_library(&mut session)` for the full Lisp-owned
    /// core bootstrap. This constructor does not invent any domain semantics.
    pub fn bare() -> Self {
        Self {
            environment: Environment::root(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        let mut session = Self {
            environment: Environment::root(),
        };
        crate::load_macro_library(&mut session)
            .expect("embedded lib/macro.lisp must bootstrap a default Session");
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoreDomainIdentity, Exactness};

    #[test]
    fn bare_session_keeps_lisp_macro_bootstrap_explicit() {
        let bare = Session::bare();
        let default = Session::default();
        assert!(
            bare.environment.get("defmacro").is_none(),
            "bare domain carrier must not silently load the Lisp macro library"
        );
        assert!(
            default.environment.get("defmacro").is_some(),
            "default Session must continue bootstrapping macro law"
        );
    }

    #[test]
    fn bare_session_evaluates_the_same_exact_d1_and_d3_results() {
        for (source, expected_bit) in [
            ("1", Some(true)),
            ("0", Some(false)),
            ("10 001 00 000 01", None),
        ] {
            let program = crate::parse_canonical_binary(source).unwrap();
            let mut bare = Session::bare();
            let mut default = Session::default();
            let left = crate::eval_parsed_expressions(&program, &mut bare).unwrap();
            let right = crate::eval_parsed_expressions(&program, &mut default).unwrap();
            assert_eq!(left.value, right.value);
            assert_eq!(left.value.as_predicate_bit(), expected_bit);
            assert_eq!(left.output, right.output);
        }
    }

    #[test]
    fn domain_code_slots_keep_equal_payloads_distinct() {
        use crate::{Bija3, Bit3, Bit4, CoreD4};

        let env = Environment::root();
        let d3 = CoreDomainIdentity::from(Bija3::from_word(Bit3::new(1).unwrap()));
        let d4 = CoreDomainIdentity::from(CoreD4::from_word(Bit4::new(1).unwrap()));

        assert!(env.bind_domain_code_slot_once(
            d3,
            Value::Symbol(Rc::from("d3-one"))
        ));
        assert!(env.bind_domain_code_slot_once(
            d4,
            Value::Symbol(Rc::from("d4-one"))
        ));

        assert_eq!(
            env.domain_code_slot(d3),
            Some(Value::Symbol(Rc::from("d3-one")))
        );
        assert_eq!(
            env.domain_code_slot(d4),
            Some(Value::Symbol(Rc::from("d4-one")))
        );
    }

    #[test]
    fn historical_w8_slots_never_alias_ratified_d8_even_with_equal_bits() {
        use crate::{Bit8, CoreD8, CoreDomainIdentity};

        let environment = Environment::root();
        let legacy = CoreDomainIdentity::from_packed_byte(0b1010_1000);
        let current = DomainIdentity::D8(CoreD8::from_word(
            Bit8::new(0b1010_1000).expect("точно вісім біт"),
        ));
        let old_value = Value::Symbol(Rc::from("історичне"));
        let new_value = Value::Symbol(Rc::from("доменне"));

        assert!(environment.bind_code_slot_once(legacy, old_value.clone()));
        assert_eq!(environment.code_slot(legacy), Some(old_value.clone()));
        assert_eq!(environment.domain_code_slot(current), None);
        assert!(environment.bind_domain_code_slot_once(current, new_value.clone()));
        assert_eq!(environment.domain_code_slot(current), Some(new_value));
        assert_eq!(environment.code_slot(legacy), Some(old_value));
        assert!(!environment.bind_code_slot_once(legacy, Value::Nil));
        assert!(!environment.bind_domain_code_slot_once(current, Value::Nil));
    }

    /// Exhaustive mechanical stress witness, not 2046 callable functions.
    /// Every (width,payload) is distinct. Current semantic DomainIdentity
    /// covers D1–D9 (1022 keys); D10's 1024 slots remain carrier-only.
    #[test]
    fn all_2046_coordinates_preserve_width_and_both_bind_once_keyspaces() {
        use crate::{
            domain_ladder::DomainCoordinate, BinarySourceWord, Bit1, Bit2, Bit3,
            Bit4, Bit5, Bit6, Bit7, Bit8, Bit9, CoreDomainIdentity,
        };
        use std::collections::HashSet;

        let environment = Environment::root();
        let mut coordinates = HashSet::new();
        let mut examined = 0usize;
        let mut significant_bits = 0usize;
        let mut current_slots = 0usize;

        for width in 1u8..=10 {
            for bits in 0..(1u16 << width) {
                let coordinate = DomainCoordinate::new(width, bits)
                    .expect("in-range exact coordinate");
                assert!(coordinates.insert(coordinate),
                    "coordinate collision at D{width}:{bits:010b}");
                assert_eq!(coordinate.width(), width);
                assert_eq!(coordinate.bits(), bits);
                examined += 1;
                significant_bits += usize::from(width);

                // The W1–W9 carrier is exact and typed; there is
                // deliberately NO lossy D10 → W8/W9 coercion.
                let source = match width {
                    1 => Some(BinarySourceWord::W1(Bit1::new(bits as u8).unwrap())),
                    2 => Some(BinarySourceWord::W2(Bit2::new(bits as u8).unwrap())),
                    3 => Some(BinarySourceWord::W3(Bit3::new(bits as u8).unwrap())),
                    4 => Some(BinarySourceWord::W4(Bit4::new(bits as u8).unwrap())),
                    5 => Some(BinarySourceWord::W5(Bit5::new(bits as u8).unwrap())),
                    6 => Some(BinarySourceWord::W6(Bit6::new(bits as u8).unwrap())),
                    7 => Some(BinarySourceWord::W7(Bit7::new(bits as u8).unwrap())),
                    8 => Some(BinarySourceWord::W8(Bit8::new(bits as u8).unwrap())),
                    9 => Some(BinarySourceWord::W9(Bit9::new(bits).unwrap())),
                    10 => None,
                    _ => unreachable!(),
                };
                let Some(source) = source else {
                    assert_eq!(width, 10);
                    // Owner residency is not DomainIdentity callability.
                    assert_eq!(coordinate.owner_residency(), Some(true));
                    continue;
                };
                let identity = DomainIdentity::from_source_word(source);
                assert_eq!(
                    DomainCoordinate::from_domain_identity(identity), coordinate,
                    "lost width on D{width}:{bits:010b}"
                );
                let marker = Value::Symbol(Rc::from(format!("D{width}:{bits}")));
                assert!(environment.bind_domain_code_slot_once(identity, marker.clone()),
                    "unexpected prebound coordinate D{width}:{bits:010b}");
                assert!(!environment.bind_domain_code_slot_once(identity, Value::Nil),
                    "overwrote exact identity D{width}:{bits:010b}");
                assert_eq!(environment.domain_code_slot(identity), Some(marker),
                    "key or payload collision D{width}:{bits:010b}");
                current_slots += 1;
            }
        }
        assert_eq!(examined, 2046);
        assert_eq!(significant_bits, 18_434);
        assert_eq!(coordinates.len(), 2046);
        assert_eq!(current_slots, 1022);

        // Every historical W8 byte has a current D8 coordinate with the
        // same low eight payload bits, but MUST NOT alias the current slot.
        for bits in 0u16..256 {
            let sid = CoreDomainIdentity::from_packed_byte(bits as u8);
            let identity = DomainIdentity::from_source_word(
                BinarySourceWord::W8(Bit8::new(bits as u8).unwrap()));
            let current = environment.domain_code_slot(identity);
            assert_eq!(environment.code_slot(sid), None,
                "historical W8 silently lifted to D8 at {bits:08b}");
            let historical = Value::Symbol(Rc::from(format!("legacy-W8:{bits}")));
            assert!(environment.bind_code_slot_once(sid, historical.clone()));
            assert!(!environment.bind_code_slot_once(sid, Value::Nil));
            assert_eq!(environment.code_slot(sid), Some(historical));
            assert_eq!(environment.domain_code_slot(identity), current,
                "historical W8 overwrote current D8 at {bits:08b}");
        }
        assert_eq!(environment.2.borrow().domain_code_slots.len(), 1022);
        assert_eq!(environment.2.borrow().legacy_code_slots.len(), 256);
        assert!(DomainCoordinate::new(10, 1024).is_none());
        assert!(DomainCoordinate::new(11, 0).is_none());
    }

    #[test]
    fn define_then_get_returns_the_value() {
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));
        assert_eq!(root.get("x"), Some(Value::Number(1.0, Exactness::Exact)));
    }

    #[test]
    fn get_on_unknown_name_returns_none() {
        let root = Environment::root();
        assert_eq!(root.get("does-not-exist"), None);
    }

    #[test]
    fn dropping_a_very_deep_environment_chain_does_not_overflow_the_stack() {
        let mut current = Environment::root();
        for _ in 0..300_000 {
            current = current.child();
        }
        drop(current);
    }

    #[test]
    fn child_reads_bindings_from_its_parent() {
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));
        let child = root.child();
        assert_eq!(child.get("x"), Some(Value::Number(1.0, Exactness::Exact)));
    }

    #[test]
    fn child_definitions_do_not_leak_into_the_parent() {
        let root = Environment::root();
        let child = root.child();
        child.define("local", Value::Number(2.0, Exactness::Exact));
        assert_eq!(root.get("local"), None);
    }

    #[test]
    fn reparent_changes_only_inherited_bindings_and_keeps_local_bindings() {
        let root = Environment::root();
        let first_parent = root.child();
        first_parent.define("surface", Value::Symbol(Rc::from("first")));
        let user = first_parent.child();
        user.define("mine", Value::Number(7.0, Exactness::Exact));

        let second_parent = root.child();
        second_parent.define("surface", Value::Symbol(Rc::from("second")));
        user.reparent(second_parent).expect("safe sibling reparent");

        assert_eq!(user.get("surface"), Some(Value::Symbol(Rc::from("second"))));
        assert_eq!(user.get("mine"), Some(Value::Number(7.0, Exactness::Exact)));
    }

    #[test]
    fn reparent_rejects_a_cycle() {
        let root = Environment::root();
        let child = root.child();
        assert!(root.reparent(child).is_err());
    }

    #[test]
    fn child_binding_shadows_the_parent_without_mutating_it() {
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));
        let child = root.child();
        child.define("x", Value::Number(2.0, Exactness::Exact));
        assert_eq!(child.get("x"), Some(Value::Number(2.0, Exactness::Exact)));
        assert_eq!(root.get("x"), Some(Value::Number(1.0, Exactness::Exact)));
    }

    #[test]
    fn redefining_in_the_same_frame_overwrites_the_previous_value() {
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));
        root.define("x", Value::Number(2.0, Exactness::Exact));
        assert_eq!(root.get("x"), Some(Value::Number(2.0, Exactness::Exact)));
    }

    /// Research-only #2402 countermodel.
    ///
    /// This deliberately lives inside the test module so no production SET/SETQ
    /// mechanism or language-visible mutation API is introduced. It walks the
    /// same Environment graph that closures capture and updates the nearest
    /// already-existing binding instead of constructing a shadow.
    fn update_nearest_existing_for_research(
        environment: &Environment,
        name: &str,
        value: Value,
    ) -> bool {
        let mut frame = Rc::clone(&environment.0);

        loop {
            let parent = {
                let mut current = frame.borrow_mut();

                if let Some(index) = current.slot_index(name) {
                    current.slots[index] = value;
                    return true;
                }

                if let Some(existing) = current.values.get_mut(name) {
                    *existing = value;
                    return true;
                }

                current
                    .parent
                    .as_ref()
                    .map(|parent| Rc::clone(&parent.0))
            };

            match parent {
                Some(parent) => frame = parent,
                None => return false,
            }
        }
    }

    #[test]
    fn nearest_existing_update_is_observably_different_from_child_define_shadowing() {
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));

        // A closure captures exactly an Environment handle. The body is not
        // needed for this lower-bound witness: the observable under test is
        // which captured binding/location the closure would resolve.
        let observer = crate::Closure {
            parameters: Vec::new(),
            rest: None,
            body: Rc::from(Vec::<crate::syntax::Expr>::new()),
            environment: root.clone(),
            slot_names: Rc::from(Vec::<Rc<str>>::new()),
            pure: true,
            resolved: None,
        };

        let child = root.child();
        child.define("x", Value::Number(2.0, Exactness::Exact));

        assert_eq!(
            child.get("x"),
            Some(Value::Number(2.0, Exactness::Exact)),
            "D4 child DEFINE sees its new shadow"
        );
        assert_eq!(
            observer.environment.get("x"),
            Some(Value::Number(1.0, Exactness::Exact)),
            "a closure created before the child shadow still sees the parent location"
        );

        // Independent graph with the same initial shape, but the candidate
        // operation updates the nearest existing location instead of creating
        // a child binding.
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));
        let observer = crate::Closure {
            parameters: Vec::new(),
            rest: None,
            body: Rc::from(Vec::<crate::syntax::Expr>::new()),
            environment: root.clone(),
            slot_names: Rc::from(Vec::<Rc<str>>::new()),
            pure: true,
            resolved: None,
        };
        let child = root.child();

        assert!(update_nearest_existing_for_research(
            &child,
            "x",
            Value::Number(2.0, Exactness::Exact)
        ));

        assert_eq!(
            child.get("x"),
            Some(Value::Number(2.0, Exactness::Exact)),
            "the caller sees NEW through inherited lookup"
        );
        assert_eq!(
            observer.environment.get("x"),
            Some(Value::Number(2.0, Exactness::Exact)),
            "the pre-existing closure sees NEW because the same parent location changed"
        );
        assert!(
            !child.0.borrow().values.contains_key("x"),
            "nearest-existing mutation must not create a child shadow"
        );
    }

    #[test]
    fn nearest_existing_update_chooses_the_nearest_location_in_a_deep_chain() {
        let root = Environment::root();
        root.define("x", Value::Number(1.0, Exactness::Exact));

        let middle = root.child();
        middle.define("x", Value::Number(2.0, Exactness::Exact));

        let leaf = middle.child();

        assert!(update_nearest_existing_for_research(
            &leaf,
            "x",
            Value::Number(3.0, Exactness::Exact)
        ));

        assert_eq!(
            leaf.get("x"),
            Some(Value::Number(3.0, Exactness::Exact))
        );
        assert_eq!(
            middle.get("x"),
            Some(Value::Number(3.0, Exactness::Exact)),
            "nearest existing middle location changed"
        );
        assert_eq!(
            root.get("x"),
            Some(Value::Number(1.0, Exactness::Exact)),
            "farther shadowed parent location is untouched"
        );
        assert!(
            !leaf.0.borrow().values.contains_key("x"),
            "no leaf binding is created"
        );
    }

    #[test]
    fn nearest_existing_update_fails_closed_when_no_binding_exists() {
        let root = Environment::root();
        let leaf = root.child().child();

        assert!(!update_nearest_existing_for_research(
            &leaf,
            "missing",
            Value::Number(9.0, Exactness::Exact)
        ));
        assert_eq!(leaf.get("missing"), None);
        assert_eq!(root.get("missing"), None);
    }

    #[test]
    fn host_policies_are_unrestricted_by_default_and_shared_with_children() {
        let root = Environment::root();
        assert!(root.fs_read_roots().is_none());
        assert!(root.fs_write_roots().is_none());
        assert!(root.is_tcp_connect_allowed("example.org", 443));
        assert!(root.is_tcp_listen_allowed("0.0.0.0", 9999));

        let root = root
            .with_fs_read_roots(vec![PathBuf::from("/safe/read")])
            .with_fs_write_roots(vec![PathBuf::from("/safe/write")])
            .with_tcp_connect_allowlist(vec![("127.0.0.1".into(), 8000, 9000)])
            .with_tcp_listen_allowlist(vec![("127.0.0.1".into(), 9999, 9999)]);
        let child = root.child();

        assert_eq!(
            child.fs_read_roots(),
            Some(vec![PathBuf::from("/safe/read")])
        );
        assert_eq!(
            child.fs_write_roots(),
            Some(vec![PathBuf::from("/safe/write")])
        );
        assert!(child.is_tcp_connect_allowed("127.0.0.1", 8080));
        assert!(!child.is_tcp_connect_allowed("example.org", 8080));
        assert!(child.is_tcp_listen_allowed("127.0.0.1", 9999));
        assert!(!child.is_tcp_listen_allowed("0.0.0.0", 9999));
    }
}

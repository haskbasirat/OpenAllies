PHASE 3 RULE SOURCES AND AUTHORED ACTOR FIELDS
===========================================

Purpose
-------
Resolve actual modified-v14 Thing definitions for inspection and later ports.
These are authored definition fields, never current runtime health, movement,
damage or simulation state. No invented combat constants are introduced.
Original files and the accepted v14 runtime remain read-only.

Client API
----------
  use aa_rules::actors::{ActorCatalog, FieldResolution};
  let catalog = ActorCatalog::load_v14(&mut verified_v14_sources)?;
  if let Some(actor) = catalog.find("usa_infantry") {
      let field = actor.field("BodyComponent.health").unwrap();
      // field.raw_text() == Some("200")
      // field.source() supplies the actual TGI path, SHA256 and property span.
  }

Other projected paths: category, name, View.resource_prefix,
BodyComponent.defense, MoverComponent.speed_max. Every field is one of Value,
Declared, Missing, or Unsupported. Value retains original expression bytes;
raw_text is an optional UTF-8 convenience. It does not remove quotes, translate
localization keys, coerce numerics, evaluate expressions, or calculate runtime
state. A declaration without an authored assignment is not invented as zero.
template_chain records most-specific to least-specific root definition sources.

load_v14 reads hash-pinned whole-file sources using the existing V14Sources
overlay resolver. All 23 accepted Data overrides remain honored. It builds a
one-time in-memory catalog suitable for the client's async cached inspector.
from_files accepts caller-supplied inputs for fixtures and verifies each
declared SHA256 against its bytes. Caller-configurable source/parser work
budgets are not a global game memory or VRAM allowance.

Supported resolution subset
---------------------------
Native CopyValuesAndSubNodesTo at 005B57C0 clones a base field only if the
destination field is absent/null. A present derived field wins. Native 005B6190
recursively fills an existing ordinary child, clones an absent ordinary child,
and does not instantiate an absent optional child on a non-template Thing.
00497220/004247C0 compare field/node keys case-insensitively. Rust uses the
ASCII subset used by this catalog and preserves original identifier spelling.

The projection follows one unambiguous Template/Inherit ancestry and unique
requested child paths. It rejects cycles, duplicate definitions/fields/children,
list semantics, unknown header operations, and multiple explicit child-template
replacement chains. It is a bounded missing-value projection, not a general
native parser/evaluator or complete template type checker. Strict-template
mandatory-field validation and native default object values are not emulated.

The catalog scans TGI sources for unique definitions, not the full index*.lst
load/evaluation schedule. Source-discovery order is not a native registry index.
Ambiguous loaded/obsolete definitions remain Unsupported rather than silently
choosing a winner. Expression text remains raw. A missing actor entry can also
mean its source syntax is unsupported; inspect catalog.diagnostics.

Grammar extension
-----------------
Document::parse remains strict and preserves its former rejection behavior.
Document::parse_native(source, limits) adds two proven plain-source tolerances:
  - a header ends at its physical line boundary even without closing ']';
  - ordinary open bodies finalize when the stream ends.
Source bytes are unchanged. Document.notices identifies each line/EOF tolerance.
Native 005BDE30 scans until ']' or current line length; 005BD080 supplies physical
lines and calls 005BAEB0 at stream EOF. The latter drains the parent stack.
Language-specific finalization, general error recovery, delayed braces, and
malformed quoted-value interpretation are not claimed.

Original corpus results
-----------------------
1,845 of 1,847 TGI files parse in the new native subset (previously 1,835).
Ten affected files are listed with original hashes and notices in
corpus-and-actors.json. Two remain explicit errors:
  Game/trigger_elements.tgi (adjacent/unbalanced quote behavior)
  Units/USA/usa_carrier.tgi (body begins after intervening properties)

The catalog exposes 1,645 Thing rows and 6,535 authored field values. Ambiguous
EasternEuropeCity02 / AARUSRegimentParatrooper templates and duplicate city-center
IDS remain explicit errors. This is not a claim that all native actor types or
all fields have been resolved. Known USA infantry 200/1/2.5, USA medium tank
400/6/3.5 and GBR Corps 3500 health match their exact original source spans.
Absent Corps MoverComponent stays Missing.

Validation
----------
native_grammar_oracle.py runs 20 original x86 cases against pinned original and
accepted v14 bytes: header scan, plain EOF stack drain, ordinary/optional child
merge, and missing-field copy. Allocator/retain/trim, name lookup, recursive
copy/clone and insertion seams are explicitly recorded in its JSON. It does
not execute a whole native loader or a general expression/type evaluator.

Rust fixtures cover source integrity, derived precedence, optional absence,
raw expressions, cycle/duplicate rejection and all new grammar notices.
phase3_local.rs additionally reads the actual pinned archive/overlay corpus,
checks all remaining failures and verifies actor values against source spans.
No original file, native executable, profile or gameplay setting is written.

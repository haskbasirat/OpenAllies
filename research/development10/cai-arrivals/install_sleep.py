from pathlib import Path
p=Path('crates/aa-runtime/src/stored_mind_init_world.rs');s=p.read_text();needle='mod regiment_move;';assert needle in s and 'mod regiment_sleep;'not in s;s=s.replace(needle,needle+'\n#[path="stored_regiment_sleep.rs"]\nmod regiment_sleep;',1);s+=Path('research/development10/cai-arrivals/think-entry-fragment.rs').read_text();p.write_text(s)
p=Path('crates/aa-runtime/src/mind_actions.rs');s=p.read_text();needle='fn organization_status(actors: &ActorAccess';assert needle in s;s=s.replace(needle,'pub(crate) fn organization_status(actors: &ActorAccess',1);p.write_text(s)

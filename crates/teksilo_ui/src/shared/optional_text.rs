//! A writable text binding for an optional string stored by a generated handle.
use std::rc::Rc;
use teksilo::core::signal::ObserverHandle;
use teksilo::prelude::Signal;

#[derive(Clone)]
pub struct OptionalText(Rc<Inner>);

struct Inner {
    text: Signal<String>,
    _observers: [ObserverHandle; 2],
}

impl OptionalText {
    pub fn new(source: Signal<Option<String>>) -> Self {
        let text = Signal::new(source.get().unwrap_or_default());
        let weak_text = text.downgrade().expect("writable text signal");
        let from_source = source.observe(move |value| {
            if let Some(text) = weak_text.upgrade() {
                text.set_if_changed(value.clone().unwrap_or_default());
            }
        });
        let weak_source = source.downgrade().expect("writable optional field");
        let to_source = text.observe(move |value| {
            if let Some(source) = weak_source.upgrade() {
                source.set_if_changed((!value.trim().is_empty()).then(|| value.clone()));
            }
        });
        Self(Rc::new(Inner {
            text,
            _observers: [from_source, to_source],
        }))
    }

    pub fn signal(&self) -> Signal<String> {
        self.0.text.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_refreshes_flow_in_both_directions() {
        let source = Signal::new(Some("Before".to_string()));
        let binding = OptionalText::new(source.clone());
        binding.signal().set("Typed".to_string());
        assert_eq!(source.get().as_deref(), Some("Typed"));
        source.set(Some("Reloaded".to_string()));
        assert_eq!(binding.signal().get(), "Reloaded");
        binding.signal().set(String::new());
        assert_eq!(source.get(), None);
    }
}

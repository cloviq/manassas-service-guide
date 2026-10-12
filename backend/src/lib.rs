use serde::Deserialize;

#[derive(Deserialize)]
pub struct TicketInput {
    pub description: String,
    pub service_id: Option<String>,
    pub contact_email: Option<String>,
}
impl TicketInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(10..=4000).contains(&self.description.trim().chars().count()) {
            return Err("Description must contain 10–4000 characters.");
        }
        if let Some(email) = &self.contact_email {
            let parts: Vec<_> = email.split('@').collect();
            if email.len() > 254
                || email.chars().any(|c| c.is_whitespace() || c.is_control())
                || parts.len() != 2
                || parts[0].is_empty()
                || !parts[1].contains('.')
                || parts[1].split('.').any(str::is_empty)
            {
                return Err("Enter a valid email address or omit it.");
            }
        }
        if self
            .service_id
            .as_ref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err("Invalid service ID.");
        }
        Ok(())
    }
}

pub mod api;
pub mod auth;
pub mod catalog;
pub mod map;
pub mod projection;
pub mod registry;
pub mod search;
pub use api::router as app;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_ticket_fields_are_validated() {
        let mut t = TicketInput {
            description: "Missed recycling collection".into(),
            service_id: None,
            contact_email: None,
        };
        for email in ["bad", "a@@b.com", "a@b", "a b@c.com", "a@b.com\n"] {
            t.contact_email = Some(email.into());
            assert!(t.validate().is_err(), "accepted invalid email");
        }
        t.contact_email = Some("resident@example.org".into());
        assert!(t.validate().is_ok());
        t.service_id = Some("not-a-uuid".into());
        assert!(t.validate().is_err());
    }
    #[test]
    fn ticket_requires_meaningful_bounded_description() {
        for description in ["", "   ", "short"] {
            let ticket = TicketInput {
                description: description.into(),
                service_id: None,
                contact_email: None,
            };
            assert!(ticket.validate().is_err());
        }
        let ticket = TicketInput {
            description: "x".repeat(4001),
            service_id: None,
            contact_email: None,
        };
        assert!(ticket.validate().is_err());
        let ticket = TicketInput {
            description: "Missed recycling collection".into(),
            service_id: None,
            contact_email: None,
        };
        assert!(ticket.validate().is_ok());
    }
}

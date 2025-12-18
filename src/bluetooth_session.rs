use dbus::{BusType, ConnMsgs, Connection};

use std::error::Error;

static BLUEZ_MATCH: &'static str = "type='signal',sender='org.bluez'";

#[derive(Debug)]
pub struct BluetoothSession {
    connection: Connection,
}

impl BluetoothSession {
    pub fn create_session(path: Option<&str>) -> Result<BluetoothSession, Box<Error>> {
        let rule = {
            if let Some(path) = path {
                format!("{},path='{}'", BLUEZ_MATCH, path)
            } else {
                String::from(BLUEZ_MATCH)
            }
        };

        let c = try!(Connection::get_private(BusType::System));
        c.add_match(rule.as_str())?;
        Ok(BluetoothSession::new(c))
    }

    /// Create a BluetoothSession without any match rules
    /// This is useful for agents that need to receive method calls, not just signals
    pub fn create_session_unfiltered() -> Result<BluetoothSession, Box<Error>> {
        let c = try!(Connection::get_private(BusType::System));
        Ok(BluetoothSession::new(c))
    }

    fn new(connection: Connection) -> BluetoothSession {
        BluetoothSession {
            connection: connection,
        }
    }

    /// Create a BluetoothSession from an existing D-Bus connection
    /// This is useful for agents that need to receive method calls, not just signals
    pub fn from_connection(connection: Connection) -> BluetoothSession {
        BluetoothSession::new(connection)
    }

    pub fn get_connection(&self) -> &Connection {
        &self.connection
    }

    pub fn incoming(&self, timeout_ms: u32) -> ConnMsgs<&Connection> {
        self.connection.incoming(timeout_ms)
    }
}

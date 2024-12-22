use dbus::{BusType, ConnMsgs, Connection};

use std::error::Error;

static BLUEZ_MATCH: &'static str = "type='signal',sender='org.bluez'";
static AGENT_PATH: &'static str = "type='signal',sender='org.bluez.Agent1'";

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
        println!("I am HEREEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE");
        // let agent_rule = String::from(AGENT_PATH);
        // let c = Connection::get_private(BusType::System)?;
        // c.add_match(agent_rule.as_str())?;
        
        let c = try!(Connection::get_private(BusType::System));
        c.add_match(rule.as_str())?;
        Ok(BluetoothSession::new(c))
    }

    fn new(connection: Connection) -> BluetoothSession {
        BluetoothSession {
            connection: connection,
        }
    }

    pub fn get_connection(&self) -> &Connection {
        &self.connection
    }

    pub fn incoming(&self, timeout_ms: u32) -> ConnMsgs<&Connection> {
        self.connection.incoming(timeout_ms)
    }
}

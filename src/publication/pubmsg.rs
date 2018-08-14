//! Common components for publication protocol messages

use std::io;
use uri;
use publication::query::{ListQuery, PublishQuery};
use publication::reply::{ListReply, SuccessReply};
use remote::xml::{AttributesError, XmlReader, XmlReaderErr, XmlWriter};
use publication::reply::ErrorReply;
use remote::xml::XmlWriterError;

pub const VERSION: &'static str = "4";
pub const NS: &'static str = "http://www.hactrn.net/uris/rpki/publication-spec/";


//------------ QueryMessage --------------------------------------------------

/// This type represents query type Publication Messages defined in RFC8181
#[derive(Debug, Eq, PartialEq)]
pub enum QueryMessage {
    PublishQuery(PublishQuery),
    ListQuery(ListQuery)
}

impl QueryMessage {

    fn decode<R>(r: &mut XmlReader<R>) -> Result<Self, MessageError>
        where R: io::Read {
        match r.next_start_name() {
            Some("list") =>{
                Ok(QueryMessage::ListQuery(ListQuery::decode(r)?))
            },
            Some("publish") | Some("withdraw") => {
                Ok(QueryMessage::PublishQuery(PublishQuery::decode(r)?))
            },
            _ => {
                Err(MessageError::ExpectedStart(
                    "list, publish, or withdraw".to_string()))
            }
        }
    }

    pub fn encode_vec<W: io::Write>(&self, w: &mut XmlWriter<W>)
        -> Result<(), XmlWriterError> {

        match self {
            QueryMessage::PublishQuery(q) => { q.encode_vec(w)?; }
            QueryMessage::ListQuery(l)    => { l.encode_vec(w)?; }
        }
        Ok(())
    }

}


//------------ ReplyMessage --------------------------------------------------

/// This type represents reply type Publication Messages defined in RFC8181
#[derive(Debug, Eq, PartialEq)]
pub enum ReplyMessage {
    SuccessReply(SuccessReply),
    ListReply(ListReply),
    ErrorReply(ErrorReply)
}

impl ReplyMessage {

    fn decode<R>(r: &mut XmlReader<R>) -> Result<Self, MessageError>
        where R: io::Read {
        match r.next_start_name() {
            Some("success") => {
                Ok(ReplyMessage::SuccessReply(SuccessReply::decode(r)?))
            },
            Some("list") => {
                Ok(ReplyMessage::ListReply(ListReply::decode(r)?))
            },
            Some("report_error") => {
                Ok(ReplyMessage::ErrorReply(ErrorReply::decode(r)?))
            },
            _ => Err(MessageError::ExpectedStart(
                "success, list or report_error".to_string()))
        }
    }

    pub fn encode_vec<W: io::Write>(&self, w: &mut XmlWriter<W>)
        -> Result<(), XmlWriterError> {

        match self {
            ReplyMessage::SuccessReply(s) => { s.encode_vec(w)?; }
            ReplyMessage::ListReply(l)    => { l.encode_vec(w)?; }
            ReplyMessage::ErrorReply(e)   => { e.encode_vec(w)?; }
        }
        Ok(())
    }

}


//------------ Message -------------------------------------------------------

/// This type represents all Publication Messages defined in RFC8181
#[derive(Debug, Eq, PartialEq)]
pub enum Message {
    QueryMessage(QueryMessage),
    ReplyMessage(ReplyMessage)
}

impl Message {

    /// Decodes an XML structure
    pub fn decode<R>(reader: R) -> Result<Self, MessageError>
        where R: io::Read {

        XmlReader::decode(reader, |r| {
            r.take_named_element("msg", |mut a, r| {

                match a.take_req("version")?.as_ref() {
                    VERSION => { },
                    _ => return Err(MessageError::InvalidVersion)
                }
                let msg_type = a.take_req("type")?;
                a.exhausted()?;

                match msg_type.as_ref() {
                    "query" => {
                        Ok(Message::QueryMessage(QueryMessage::decode(r)?))
                    },
                    "reply" => {
                        Ok(Message::ReplyMessage(ReplyMessage::decode(r)?))
                    }
                    _ => {
                        return Err(MessageError::UnknownMessageType)
                    }
                }
            })
        })
    }

    /// Encodes to a Vec
    pub fn encode_vec(&self) -> Vec<u8> {
        XmlWriter::encode_vec(|w| {

            let msg_type = match self {
                Message::QueryMessage(_) => "query",
                Message::ReplyMessage(_) => "reply"
            };
            let a = [
                ("xmlns", NS),
                ("version", VERSION),
                ("type", msg_type),
            ];

            w.put_element(
                "msg",
                Some(&a),
                |w| {
                    match self {
                        Message::ReplyMessage(r) => { r.encode_vec(w) }
                        Message::QueryMessage(q) => { q.encode_vec(w) }
                    }
                }
            )
        })
    }

}

//------------ PublicationMessageError ---------------------------------------

#[derive(Debug, Fail)]
pub enum MessageError {

    #[fail(display = "Invalid version")]
    InvalidVersion,

    #[fail(display = "Unknown message type")]
    UnknownMessageType,

    #[fail(display = "Unexpected XML Start Tag: {}", _0)]
    UnexpectedStart(String),

    #[fail(display = "Expected some XML Start Tag: {}", _0)]
    ExpectedStart(String),

    #[fail(display = "Missing content in XML: {}", _0)]
    MissingContent(String),

    #[fail(display = "Invalid XML file: {}", _0)]
    XmlReadError(XmlReaderErr),

    #[fail(display = "Invalid use of attributes in XML file: {}", _0)]
    XmlAttributesError(AttributesError),

    #[fail(display = "Invalid URI: {}", _0)]
    UriError(uri::Error),
}

impl From<XmlReaderErr> for MessageError {
    fn from(e: XmlReaderErr) -> MessageError {
        MessageError::XmlReadError(e)
    }
}

impl From<AttributesError> for MessageError {
    fn from(e: AttributesError) -> MessageError {
        MessageError::XmlAttributesError(e)
    }
}

impl From<uri::Error> for MessageError {
    fn from(e: uri::Error) -> MessageError {
        MessageError::UriError(e)
    }
}


//------------ Tests ---------------------------------------------------------

#[cfg(test)]
mod tests {

    use super::*;
    use std::str;

    fn assert_re_encode_equals(object: Message, xml: &str) {
        let vec = object.encode_vec();
        let encoded_xml = str::from_utf8(&vec).unwrap();
        let object_from_encoded_xml = Message::decode(encoded_xml.as_bytes()).unwrap();
        assert_eq!(object, object_from_encoded_xml);
        assert_eq!(xml, encoded_xml);
    }

    #[test]
    fn should_parse_and_encode_multi_element_query() {
        let xml = include_str!("../../test/publication/publish.xml");
        let pm = Message::decode(xml.as_bytes()).unwrap();
        assert_re_encode_equals(pm, xml);
    }

    #[test]
    fn should_parse_and_encode_list_query() {
        let xml = include_str!("../../test/publication/list.xml");
        let l = Message::decode(xml.as_bytes()).unwrap();
        assert_re_encode_equals(l, xml);
    }

    #[test]
    fn should_parse_and_encode_success_reply() {
        let xml = include_str!("../../test/publication/success.xml");
        let s = Message::decode(xml.as_bytes()).unwrap();
        assert_re_encode_equals(s, xml);
    }

    #[test]
    fn should_parse_and_encode_list_reply() {
        let xml = include_str!("../../test/publication/list-reply.xml");
        let r = Message::decode(xml.as_bytes()).unwrap();
        assert_re_encode_equals(r, xml);
    }

    #[test]
    fn should_parse_and_encode_minimal_error() {
        let xml = include_str!("../../test/publication/report_error_minimal.xml");
        let e = Message::decode(xml.as_bytes()).unwrap();
        assert_re_encode_equals(e, xml);
    }

    #[test]
    fn should_parse_and_encode_complex_error() {
        let xml = include_str!("../../test/publication/report_error_complex.xml");
        let e = Message::decode(xml.as_bytes()).unwrap();
        assert_re_encode_equals(e, xml);
    }

}
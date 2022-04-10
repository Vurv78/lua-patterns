// use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Error {
	/// Like doing "(hello) %4".
	/// Tried to get the fourth group but pattern only has one.
	InvalidCapture( Option<i8> ),

	/// Gone over [LUA_MAXCAPTURES]
	TooManyCaptures,

	/// Unbalanced parenthesis. "(foo)(bar"
	UnfinishedCapture,

	/// Using ) without a (
	NoOpenCapture,

	/// Stack overflow limit
	TooComplex,

	/// Ends with %
	EndsWithPercent,

	/// Missing ]
	MissingEndBracket,

	/// Missing arguments to %b. Should be like %b<>.
	MissingBalanceArgs,

	/// Missing [ bracket after %f
	MissingLBracketF,

	/// Unfinished or positional capture where not expected
	CapLen
}

/// Display Error with proper error messages you'd get from lua.
impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::InvalidCapture( Some(n) ) => write!(f, "invalid capture index %{}", n),
			Error::InvalidCapture( None ) => write!(f, "invalid pattern capture"),
			Error::TooManyCaptures => write!(f, "too many captures"),
			Error::UnfinishedCapture => write!(f, "unfinished capture"),
			Error::NoOpenCapture => write!(f, "no open capture"),
			Error::TooComplex => write!(f, "pattern too complex"),
			Error::EndsWithPercent => write!(f, "malformed pattern (ends with '%')"),
			Error::MissingEndBracket => write!(f, "malformed pattern (missing ']')"),
			Error::MissingBalanceArgs => write!(f, "malformed pattern (missing arguments to '%b')"),
			Error::MissingLBracketF => write!(f, "missing '[' after '%f' in pattern"),
			Error::CapLen => write!(f, "capture was unfinished or positional (this shouldn't happen..?)")
		}
	}
}
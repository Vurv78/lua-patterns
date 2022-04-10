use std::ops;

pub mod errors;
use errors::*;
mod pattern;
use pattern::*;

/// Represents a Lua string pattern and the results of a match
pub struct Pattern<'a> {
	patt: &'a [u8],
	matches: [LuaMatch; LUA_MAXCAPTURES],
	n_match: usize,
}

impl<'a> Pattern<'a> {
	pub fn try_from_bytes(bytes: &'a [u8]) -> Result<Pattern<'a>, PatternError> {
		str_check(bytes)?;
		let matches = [LuaMatch { start: 0, end: 0 }; LUA_MAXCAPTURES];
		Ok(Pattern {
			patt: bytes,
			matches,
			n_match: 0,
		})
	}

	pub fn new(patt: &'a str) -> Result<Pattern<'a>, PatternError> {
		Pattern::try_from_bytes(patt.as_bytes())
	}

	pub fn matches_bytes(&mut self, s: &[u8]) -> bool {
		self.n_match =
			str_match(s, self.patt, &mut self.matches).expect("Should not fail - report as bug");
		self.n_match > 0
	}

	pub fn matches(&mut self, text: &str) -> bool {
		self.matches_bytes(text.as_bytes())
	}

	pub fn match_maybe<'t>(&mut self, text: &'t str) -> Option<&'t str> {
		if self.matches(text) {
			Some(&text[self.first_capture()])
		} else {
			None
		}
	}

	pub fn match_maybe_2<'t>(&mut self, text: &'t str) -> Option<(&'t str, &'t str)> {
		if self.matches(text) {
			let cc = self.match_captures(text);
			if cc.num_matches() != 3 {
				return None;
			}
			Some((cc.get(1), cc.get(2)))
		} else {
			None
		}
	}

	pub fn match_maybe_3<'t>(&mut self, text: &'t str) -> Option<(&'t str, &'t str, &'t str)> {
		if self.matches(text) {
			let cc = self.match_captures(text);
			if cc.num_matches() != 4 {
				return None;
			}
			Some((cc.get(1), cc.get(2), cc.get(3)))
		} else {
			None
		}
	}

	pub fn captures<'b>(&mut self, text: &'b str) -> Vec<&'b str> {
		let mut res = Vec::new();
		self.capture_into(text, &mut res);
		res
	}

	pub fn match_captures<'b, 'c>(&'c self, text: &'b str) -> Captures<'a, 'b, 'c> {
		Captures { m: self, text }
	}

	pub fn capture_into<'b>(&mut self, text: &'b str, vec: &mut Vec<&'b str>) -> bool {
		self.matches(text);
		vec.clear();
		for i in 0..self.n_match {
			vec.push(&text[self.capture(i)]);
		}
		self.n_match > 0
	}

	/// The full match (same as `capture(0)`)
	pub fn range(&self) -> ops::Range<usize> {
		self.capture(0)
	}

	pub fn capture(&self, i: usize) -> ops::Range<usize> {
		ops::Range {
			start: self.matches[i].start as usize,
			end: self.matches[i].end as usize,
		}
	}

	pub fn first_capture(&self) -> ops::Range<usize> {
		let idx = if self.n_match > 1 { 1 } else { 0 };
		self.capture(idx)
	}

	pub fn gmatch<'b, 'c>(&'c mut self, text: &'b str) -> GMatch<'a, 'b, 'c> {
		GMatch { m: self, text }
	}

	pub fn gmatch_captures<'b, 'c>(&'c mut self, text: &'b str) -> GMatchCaptures<'a, 'b, 'c> {
		GMatchCaptures { m: self, text }
	}

	pub fn gmatch_bytes<'b>(&'a mut self, bytes: &'b [u8]) -> GMatchBytes<'a, 'b> {
		GMatchBytes { m: self, bytes }
	}

	pub fn gsub_with<F>(&mut self, text: &str, lookup: F) -> String
	where
		F: Fn(Captures) -> String,
	{
		let mut slice = text;
		let mut res = String::new();
		while self.matches(slice) {
			// full range of match
			let all = self.range();
			// append everything up to match
			res.push_str(&slice[0..all.start]);
			let captures = Captures {
				m: self,
				text: slice,
			};
			let repl = lookup(captures);
			res.push_str(&repl);
			slice = &slice[all.end..];
		}
		res.push_str(slice);
		res
	}

	pub fn gsub(&mut self, text: &str, repl: &str) -> Result<String, PatternError> {
		let repl = generate_gsub_patterns(repl)?;
		let mut slice = text;
		let mut res = String::new();
		while self.matches(slice) {
			let all = self.range();
			res.push_str(&slice[0..all.start]);
			let captures = Captures {
				m: self,
				text: slice,
			};
			for r in &repl {
				match *r {
					Subst::Text(ref s) => res.push_str(s),
					Subst::Capture(i) => res.push_str(captures.get(i)),
				}
			}
			slice = &slice[all.end..];
		}
		res.push_str(slice);
		Ok(res)
	}

	pub fn gsub_bytes_with<F>(&mut self, bytes: &[u8], lookup: F) -> Vec<u8>
	where
		F: Fn(ByteCaptures) -> Vec<u8>,
	{
		let mut slice = bytes;
		let mut res = Vec::new();
		while self.matches_bytes(slice) {
			let all = self.range();
			let capture = &slice[0..all.start];
			res.extend_from_slice(capture);
			let captures = ByteCaptures {
				m: self,
				bytes: slice,
			};
			let repl = lookup(captures);
			res.extend(repl);
			slice = &slice[all.end..];
		}
		res.extend_from_slice(slice);
		res
	}
}

#[derive(Debug)]
pub enum Subst {
	Text(String),
	Capture(usize),
}

impl Subst {
	fn new_text(text: &str) -> Subst {
		Subst::Text(text.to_string())
	}
}

pub fn generate_gsub_patterns(repl: &str) -> Result<Vec<Subst>, PatternError> {
	let mut m = Pattern::new("%%([%%%d])")?;
	let mut res = Vec::new();
	let mut slice = repl;
	while m.matches(slice) {
		let all = m.range();
		let before = &slice[0..all.start];
		if !before.is_empty() {
			res.push(Subst::new_text(before));
		}
		let capture = &slice[m.capture(1)];
		if capture == "%" {
			// escaped literal '%'
			res.push(Subst::new_text("%"));
		} else {
			// has to be a digit
			let index: usize = capture.parse().unwrap();
			res.push(Subst::Capture(index));
		}
		slice = &slice[all.end..];
	}
	res.push(Subst::new_text(slice));
	Ok(res)
}

pub struct Substitute {
	repl: Vec<Subst>,
}

impl Substitute {
	pub fn new(repl: &str) -> Result<Substitute, PatternError> {
		Ok(Substitute {
			repl: generate_gsub_patterns(repl)?,
		})
	}

	pub fn subst(&self, patt: &Pattern, text: &str) -> String {
		let mut res = String::new();
		let captures = patt.match_captures(text);
		for r in &self.repl {
			match *r {
				Subst::Text(ref s) => res.push_str(s),
				Subst::Capture(i) => res.push_str(captures.get(i)),
			}
		}
		res
	}
}

pub struct Captures<'a, 'b, 'c>
where
	'a: 'c,
{
	m: &'c Pattern<'a>,
	text: &'b str,
}

impl<'a, 'b, 'c> Captures<'a, 'b, 'c> {
	/// get the capture as a string slice
	pub fn get(&self, i: usize) -> &'b str {
		&self.text[self.m.capture(i)]
	}

	/// number of matches
	pub fn num_matches(&self) -> usize {
		self.m.n_match
	}
}

pub struct ByteCaptures<'a, 'b> {
	m: &'a Pattern<'a>,
	bytes: &'b [u8],
}

impl<'a, 'b> ByteCaptures<'a, 'b> {
	pub fn get(&self, i: usize) -> &'b [u8] {
		&self.bytes[self.m.capture(i)]
	}

	pub fn num_matches(&self) -> usize {
		self.m.n_match
	}
}

pub struct GMatch<'a, 'b, 'c>
where
	'a: 'c,
{
	m: &'c mut Pattern<'a>,
	text: &'b str,
}

impl<'a, 'b, 'c> Iterator for GMatch<'a, 'b, 'c> {
	type Item = &'b str;

	fn next(&mut self) -> Option<Self::Item> {
		if !self.m.matches(self.text) {
			None
		} else {
			let slice = &self.text[self.m.first_capture()];
			self.text = &self.text[self.m.range().end..];
			Some(slice)
		}
	}
}

pub struct CapturesUnsafe<'b> {
	matches: *const LuaMatch,
	text: &'b str,
}

impl<'b> CapturesUnsafe<'b> {
	/// get the capture as a string slice
	pub fn get(&self, i: usize) -> &'b str {
		unsafe {
			let p = self.matches.add(i);
			let range = ops::Range {
				start: (*p).start as usize,
				end: (*p).end as usize,
			};
			&self.text[range]
		}
	}
}

pub struct GMatchCaptures<'a, 'b, 'c>
where
	'a: 'c,
{
	m: &'c mut Pattern<'a>,
	text: &'b str,
}

impl<'a, 'b, 'c> Iterator for GMatchCaptures<'a, 'b, 'c>
where
	'a: 'c,
{
	type Item = CapturesUnsafe<'b>;

	fn next(&mut self) -> Option<Self::Item> {
		if !self.m.matches(self.text) {
			None
		} else {
			let split = self.text.split_at(self.m.range().end);
			self.text = split.1;
			let match_ptr: *const LuaMatch = self.m.matches.as_ptr();
			Some(CapturesUnsafe {
				matches: match_ptr,
				text: split.0,
			})
		}
	}
}

/// Iterator for all byte slices from `gmatch_bytes`
pub struct GMatchBytes<'a, 'b> {
	m: &'a mut Pattern<'a>,
	bytes: &'b [u8],
}

impl<'a, 'b> Iterator for GMatchBytes<'a, 'b> {
	type Item = &'b [u8];

	fn next(&mut self) -> Option<Self::Item> {
		if !self.m.matches_bytes(self.bytes) {
			None
		} else {
			let slice = &self.bytes[self.m.first_capture()];
			self.bytes = &self.bytes[self.m.range().end..];
			Some(slice)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bad_patterns() {
		let bad = [
			("%", "malformed pattern (ends with '%')"),
			("(dog%(", "unfinished capture"),
			("[%a%[", "malformed pattern (missing ']')"),
			("(()", "unfinished capture"),
			("[%A", "malformed pattern (missing ']')"),
			("(1) (2(3)%2)%1", "invalid capture index %2"),
		];
		for p in bad.iter() {
			if let Err(why) = Pattern::new(p.0) {
				assert_eq!(why, PatternError(p.1.to_owned()));
			} else {
				panic!("pattern {} should have failed", p.0);
			}
		}
	}
}

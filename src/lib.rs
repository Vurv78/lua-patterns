use std::ops;

pub mod error;
use error::*;
mod pattern;
use pattern::*;

/// Represents a Lua string pattern and the results of a match
pub struct Pattern<'a, const MAXCAPTURES: usize = LUA_MAXCAPTURES> {
	patt: &'a [u8],
	matches: [LuaMatch; MAXCAPTURES],
	n_match: usize,
}

impl<'a, const MAXCAPTURES: usize> Pattern<'a, MAXCAPTURES> {
	pub fn try_from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
		str_check::<MAXCAPTURES>(bytes)?;
		let matches = [LuaMatch { start: 0, end: 0 }; MAXCAPTURES];
		Ok(Pattern {
			patt: bytes,
			matches,
			n_match: 0,
		})
	}

	pub fn new<S: AsRef<[u8]> + ?Sized>(pattern: &'a S) -> Result<Self, Error> {
		Pattern::try_from_bytes( pattern.as_ref() )
	}

	pub fn matches_bytes(&mut self, s: &[u8]) -> bool {
		self.n_match = str_match::<MAXCAPTURES>(s, self.patt, &mut self.matches).expect("Should not fail - report as bug");
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

	pub fn match_captures<'b, 'c>(&'c self, text: &'b str) -> Captures<'a, 'b, 'c, MAXCAPTURES> {
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

	pub fn gmatch<'b, 'c>(&'c mut self, text: &'b str) -> GMatch<'a, 'b, 'c, MAXCAPTURES> {
		GMatch { m: self, text }
	}

	pub fn gmatch_captures<'b, 'c>(&'c mut self, text: &'b str) -> GMatchCaptures<'a, 'b, 'c, MAXCAPTURES> {
		GMatchCaptures { m: self, text }
	}

	pub fn gmatch_bytes<'b>(&'a mut self, bytes: &'b [u8]) -> GMatchBytes<'a, 'b, MAXCAPTURES> {
		GMatchBytes { m: self, bytes }
	}

	pub fn gsub_with<F>(&mut self, text: &str, lookup: F) -> String
	where
		F: Fn(Captures<MAXCAPTURES>) -> String,
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

	pub fn gsub(&mut self, text: &str, repl: &str) -> Result<String, Error> {
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
		F: Fn(ByteCaptures<MAXCAPTURES>) -> Vec<u8>,
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

pub fn generate_gsub_patterns(repl: &str) -> Result<Vec<Subst>, Error> {
	let mut m: Pattern<'_, 2> = Pattern::new("%%([%%%d])")?;

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
	pub fn new(repl: &str) -> Result<Self, Error> {
		Ok(Substitute {
			repl: generate_gsub_patterns(repl)?,
		})
	}

	pub fn subst<const MAXCAPTURES: usize>(&self, patt: &Pattern<MAXCAPTURES>, text: &str) -> String {
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

pub struct Captures<'a, 'b, 'c, const MAXCAPTURES: usize = LUA_MAXCAPTURES>
where
	'a: 'c,
{
	m: &'c Pattern<'a, MAXCAPTURES>,
	text: &'b str,
}

impl<'a, 'b, 'c, const MAXCAPTURES: usize> Captures<'a, 'b, 'c, MAXCAPTURES> {
	/// get the capture as a string slice
	pub fn get(&self, i: usize) -> &'b str {
		&self.text[self.m.capture(i)]
	}

	/// number of matches
	pub fn num_matches(&self) -> usize {
		self.m.n_match
	}
}

pub struct ByteCaptures<'a, 'b, const MAXCAPTURES: usize = LUA_MAXCAPTURES> {
	m: &'a Pattern<'a, MAXCAPTURES>,
	bytes: &'b [u8],
}

impl<'a, 'b, const MAXCAPTURES: usize> ByteCaptures<'a, 'b, MAXCAPTURES> {
	pub fn get(&self, i: usize) -> &'b [u8] {
		&self.bytes[self.m.capture(i)]
	}

	pub fn num_matches(&self) -> usize {
		self.m.n_match
	}
}

pub struct GMatch<'a, 'b, 'c, const MAXCAPTURES: usize = LUA_MAXCAPTURES>
where
	'a: 'c,
{
	m: &'c mut Pattern<'a, MAXCAPTURES>,
	text: &'b str,
}

impl<'a, 'b, 'c, const MAXCAPTURES: usize> Iterator for GMatch<'a, 'b, 'c, MAXCAPTURES> {
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

pub struct GMatchCaptures<'a, 'b, 'c, const MAXCAPTURES: usize = LUA_MAXCAPTURES>
where
	'a: 'c,
{
	m: &'c mut Pattern<'a, MAXCAPTURES>,
	text: &'b str,
}

impl<'a, 'b, 'c, const MAXCAPTURES: usize> Iterator for GMatchCaptures<'a, 'b, 'c, MAXCAPTURES>
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
pub struct GMatchBytes<'a, 'b, const MAXCAPTURES: usize = LUA_MAXCAPTURES> {
	m: &'a mut Pattern<'a, MAXCAPTURES>,
	bytes: &'b [u8],
}

impl<'a, 'b, const MAXCAPTURES: usize> Iterator for GMatchBytes<'a, 'b, MAXCAPTURES> {
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

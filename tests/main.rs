extern crate lupat;
use lupat::{Pattern, error::Error};

#[test]
fn bad_patterns() {
	let tests = [
		("%",               Error::EndsWithPercent),
		("(dog%(",          Error::UnfinishedCapture),
		("[%a%[",           Error::MissingEndBracket),
		("(()",             Error::UnfinishedCapture),
		("[%A",             Error::MissingEndBracket),
		("(1) (2(3)%2)%1",  Error::InvalidCapture(Some(2))),
	];

	for p in tests.iter() {
		let pattern: Result<Pattern<'_, 8>, _> = Pattern::new(p.0);
		match pattern {
			Err(why) => {
				assert_eq!(why, p.1);
			},
			Ok(_) => panic!("pattern {} should not have parsed", p.0),
		}
	}
}


#[test]
fn basic() {
	let patterns = ["%w+", "(%w+)", "([%w]+)",];
	let test_str = "test foo bar"; // All patterns should match 3 items, (test), (foo), (bar)

	for pattern in patterns {
		let mut pattern: Pattern<'_, 2> = Pattern::new(pattern).unwrap();
		assert_eq!( pattern.gmatch(test_str).count(), 3 );
	}
}

#[test]
fn stack() {
	// 50 parenthesis + 1 base group (being 0)
	let mut pattern: Pattern<'_, 51> = Pattern::new("(((((((((((((((((((((((((((((((((((((((((((((((((())))))))))))))))))))))))))))))))))))))))))))))))))").unwrap();
	pattern.matches("foo bar");

	assert_eq!( std::mem::size_of::<Pattern<'_, 0>>(), 24 );
	assert_eq!( std::mem::size_of::<Pattern<'_, 50>>(), 24 + ( /* LuaMatch is u8 x 2 */ 16 * 50) );
}
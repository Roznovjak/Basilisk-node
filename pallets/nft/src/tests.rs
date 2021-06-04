use frame_support::{assert_noop, assert_ok, error::BadOrigin};

use super::*;
use mock::{Event, *};

type NftModule = Pallet<Test>;

#[test]
fn create_class_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));
		let event = Event::pallet_nft(crate::Event::NFTTokenClassCreated(ALICE, CLASS_ID));
		assert_eq!(last_event(), event);
	})
}

#[test]
fn create_class_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			NftModule::create_class(
				Origin::none(),
				"a class".as_bytes().to_vec(),
				Default::default(),
				TEST_PRICE
			),
			BadOrigin
		);
	})
}

#[test]
fn mint_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));
		let event = Event::pallet_nft(crate::Event::NFTTokenClassCreated(ALICE, CLASS_ID));
		assert_eq!(last_event(), event);

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));
		let event = Event::pallet_nft(crate::Event::NFTTokenMinted(ALICE, CLASS_ID, TEST_QUANTITY));
		assert_eq!(last_event(), event);
	});
}

#[test]
fn mint_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));
		let event = Event::pallet_nft(crate::Event::NFTTokenClassCreated(ALICE, CLASS_ID));
		assert_eq!(last_event(), event);

		assert_noop!(
			NftModule::mint(
				Origin::signed(BOB),
				0,
				"a token".as_bytes().to_vec(),
				TokenData {
					locked: false,
					emote: EMOTE.as_bytes().to_vec()
				},
				TEST_QUANTITY,
			),
			Error::<Test>::NotClassOwner
		);
	});
}

#[test]
fn transfer_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_ok!(NftModule::transfer(Origin::signed(ALICE), BOB, (CLASS_ID, TOKEN_ID)));
		let event = Event::pallet_nft(crate::Event::NFTTokenTransferred(ALICE, BOB, CLASS_ID, TOKEN_ID));
		assert_eq!(last_event(), event);
	});
}

#[test]
fn transfer_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_noop!(
			NftModule::transfer(Origin::signed(BOB), ALICE, (CLASS_ID, TOKEN_ID)),
			Error::<Test>::NotTokenOwner
		);

		assert_noop!(
			NftModule::transfer(Origin::signed(ALICE), ALICE, (CLASS_ID, TOKEN_ID)),
			Error::<Test>::CannotSendToSelf
		);
	});
}

#[test]
fn burn_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_ok!(NftModule::burn(Origin::signed(ALICE), (CLASS_ID, TOKEN_ID)));
		let event = Event::pallet_nft(crate::Event::NFTTokenBurned(ALICE, CLASS_ID, TOKEN_ID));
		assert_eq!(last_event(), event);
	});
}

#[test]
fn burn_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_noop!(
			NftModule::burn(Origin::signed(BOB), (CLASS_ID, TOKEN_ID)),
			Error::<Test>::NotTokenOwner
		);
	});
}

#[test]
fn destroy_class_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::destroy_class(Origin::signed(ALICE), CLASS_ID));
	});
}

#[test]
fn destroy_class_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_noop!(
			NftModule::destroy_class(Origin::signed(ALICE), CLASS_ID),
			Error::<Test>::NonZeroIssuance
		);
	});
}

#[test]
fn toggle_lock_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_ok!(NftModule::toggle_lock(&ALICE, (CLASS_ID, TOKEN_ID)));
		let locked = NftModule::is_locked((CLASS_ID, TOKEN_ID));
		assert!(locked.unwrap());
	});
}

#[test]
fn toggle_lock_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_noop!(
			NftModule::toggle_lock(&BOB, (CLASS_ID, TOKEN_ID)),
			Error::<Test>::NotTokenOwner
		);
	});
}

#[test]
fn buy_from_pool_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_ok!(NftModule::buy_from_pool(Origin::signed(BOB), (CLASS_ID, TOKEN_ID)));
		let event = Event::pallet_nft(crate::Event::NFTBoughtFromPool(ALICE, BOB, CLASS_ID, TOKEN_ID));
		assert_eq!(last_event(), event);
	});
}

#[test]
fn buy_from_pool_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_ok!(NftModule::transfer(Origin::signed(ALICE), BOB, (CLASS_ID, TOKEN_ID)));

		assert_noop!(
			NftModule::buy_from_pool(Origin::signed(ALICE), (CLASS_ID, TOKEN_ID)),
			Error::<Test>::TokenAlreadyHasAnOwner
		);

		assert_ok!(NftModule::transfer(Origin::signed(BOB), ALICE, (CLASS_ID, TOKEN_ID)));

		assert_noop!(
			NftModule::buy_from_pool(Origin::signed(ALICE), (CLASS_ID, TOKEN_ID)),
			Error::<Test>::CannotBuyOwnToken
		);
	});
}

#[test]
fn sell_to_pool_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_ok!(NftModule::transfer(Origin::signed(ALICE), BOB, (CLASS_ID, TOKEN_ID)));

		assert_ok!(NftModule::sell_to_pool(Origin::signed(BOB), (CLASS_ID, TOKEN_ID)));
		let event = Event::pallet_nft(crate::Event::NFTSoldToPool(BOB, ALICE, CLASS_ID, TOKEN_ID));
		assert_eq!(last_event(), event);
	});
}

#[test]
fn sell_to_pool_fails() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NftModule::create_class(
			Origin::signed(ALICE),
			"a class".as_bytes().to_vec(),
			Default::default(),
			TEST_PRICE
		));

		assert_ok!(NftModule::mint(
			Origin::signed(ALICE),
			0,
			"a token".as_bytes().to_vec(),
			TokenData {
				locked: false,
				emote: EMOTE.as_bytes().to_vec()
			},
			TEST_QUANTITY,
		));

		assert_noop!(
			NftModule::sell_to_pool(Origin::signed(ALICE), (CLASS_ID, TOKEN_ID)),
			Error::<Test>::CannotSellPoolToken
		);

		assert_ok!(NftModule::transfer(
			Origin::signed(ALICE),
			CHARLIE,
			(CLASS_ID, TOKEN_ID)
		));

		assert_noop!(
			NftModule::sell_to_pool(Origin::signed(BOB), (CLASS_ID, TOKEN_ID)),
			Error::<Test>::NotTokenOwner
		);
	});
}

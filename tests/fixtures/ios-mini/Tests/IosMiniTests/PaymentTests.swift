import XCTest
@testable import IosMini

final class PaymentTests: XCTestCase {
    func testRejectsExpiredCard() {
        let rules = PaymentRules()

        XCTAssertFalse(rules.approves(validCard: false))
    }
}

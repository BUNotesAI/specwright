package com.example

import org.junit.Assert.assertTrue
import org.junit.Test

class PaymentRulesTest {
    @Test
    fun approvesValidCard() {
        assertTrue(PaymentRules.approvesValidCard("4111111111111111"))
    }
}

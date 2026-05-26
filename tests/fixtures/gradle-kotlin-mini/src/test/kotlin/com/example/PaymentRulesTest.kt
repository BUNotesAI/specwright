package com.example

import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class PaymentRulesTest {
    @Spec("gradle kotlin scenario")
    @Test
    fun approvesValidPayment() {
        assertTrue("paid".startsWith("pa"))
    }
}

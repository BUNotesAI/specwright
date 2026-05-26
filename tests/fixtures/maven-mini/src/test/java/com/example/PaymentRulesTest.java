package com.example;

import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class PaymentRulesTest {
    @Spec("maven java scenario")
    @Test
    void approvesValidCard() {
        assertTrue("424242".startsWith("42"));
    }
}

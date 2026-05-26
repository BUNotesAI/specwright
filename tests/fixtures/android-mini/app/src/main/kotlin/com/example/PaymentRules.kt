package com.example

object PaymentRules {
    fun approvesValidCard(pan: String): Boolean = pan.startsWith("4111") && pan.length == 16
}

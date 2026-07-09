pub fn backend_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn backend_smoke_test() {
        assert!(crate::backend_ready());
    }
}

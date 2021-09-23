/*
 * Copyright (C) 2021 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use apkverify::verify;

macro_rules! assert_contains {
    ($haystack:expr,$needle:expr $(,)?) => {
        match (&$haystack, &$needle) {
            (haystack_value, needle_value) => {
                assert!(
                    haystack_value.contains(needle_value),
                    "{} is not found in {}",
                    needle_value,
                    haystack_value
                );
            }
        }
    };
}

#[test]
fn test_verify_v3() {
    assert!(verify("tests/data/test.apex").is_ok());
}

#[test]
fn test_verify_v3_digest_mismatch() {
    let res = verify("tests/data/v3-only-with-rsa-pkcs1-sha512-8192-digest-mismatch.apk");
    assert!(res.is_err());
    assert_contains!(res.err().unwrap().to_string(), "Digest mismatch");
}

#[test]
fn test_verify_v3_cert_and_publick_key_mismatch() {
    let res = verify("tests/data/v3-only-cert-and-public-key-mismatch.apk");
    assert!(res.is_err());
    assert_contains!(res.err().unwrap().to_string(), "Public key mismatch");
}

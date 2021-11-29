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

package com.android.compos;

import com.android.compos.CompOsKeyData;
import com.android.compos.CompilationResult;
import com.android.compos.FdAnnotation;

/** {@hide} */
interface ICompOsService {
    /**
     * Initializes the service with the supplied encrypted private key blob. The key cannot be
     * changed once initialized, so once initiailzed, a repeated call will fail with
     * EX_ILLEGAL_STATE.
     *
     * @param keyBlob The encrypted blob containing the private key, as returned by
     *                generateSigningKey().
     */
    void initializeSigningKey(in byte[] keyBlob);

    /**
     * Initializes the classpaths necessary for preparing and running compilation.
     *
     * TODO(198211396): Implement properly. We can't simply accepting the classpaths from Android
     * since they are not derived from staged APEX (besides security reasons).
     */
    void initializeClasspaths(String bootClasspath, String dex2oatBootClasspath);

    /**
     * Run dex2oat command with provided args, in a context that may be specified in FdAnnotation,
     * e.g. with file descriptors pre-opened. The service is responsible to decide what executables
     * it may run.
     *
     * @param args The command line arguments to run. The 0-th args is normally the program name,
     *             which may not be used by the service. The service may be configured to always use
     *             a fixed executable, or possibly use the 0-th args are the executable lookup hint.
     * @param fd_annotation Additional file descriptor information of the execution
     * @return a CompilationResult
     */
    CompilationResult compile_cmd(in String[] args, in FdAnnotation fd_annotation);

    /**
     * Runs dexopt compilation encoded in the marshaled dexopt arguments.
     *
     * To keep ART indepdendantly updatable, the compilation arguments are not stabilized. As a
     * result, the arguments are marshaled into byte array.  Upon received, the service asks ART to
     * return relevant information (since ART is able to unmarshal its own encoding), in order to
     * set up the execution context (mainly file descriptors for compiler input and output) then
     * invokes the compiler.
     *
     * @param marshaledArguments The marshaled dexopt arguments.
     * @param fd_annotation Additional file descriptor information of the execution.
     * @return exit code
     */
    byte compile(in byte[] marshaledArguments, in FdAnnotation fd_annotation);

    /**
     * Generate a new public/private key pair suitable for signing CompOs output files.
     *
     * @return a certificate for the public key and the encrypted private key
     */
    CompOsKeyData generateSigningKey();

    /**
     * Check that the supplied encrypted private key is valid for signing CompOs output files, and
     * corresponds to the public key.
     *
     * @param keyBlob The encrypted blob containing the private key, as returned by
     *                generateSigningKey().
     * @param publicKey The public key, as a DER encoded RSAPublicKey (RFC 3447 Appendix-A.1.1).
     * @return whether the inputs are valid and correspond to each other.
     */
    boolean verifySigningKey(in byte[] keyBlob, in byte[] publicKey);

    /**
     * Signs some data with the initialized key. The call will fail with EX_ILLEGAL_STATE if not
     * yet initialized.
     *
     * @param data The data to be signed. (Large data sizes may cause failure.)
     * @return the signature.
     */
    // STOPSHIP(b/193241041): We must not expose this from the PVM.
    byte[] sign(in byte[] data);
}

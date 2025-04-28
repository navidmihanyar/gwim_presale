
# Program Source Verification Proof

This document certifies that the deployed program on Solana matches exactly the locally built source code based on hash verification.

## Program Information

- **Program Name:** gwimanchor
- **Program ID:** GiCazGBqZBQEJz5CipbMMZiWfZX9FRE9kXLtwQNS2Vsj
- **Network:** Solana Mainnet

## Local Build Information

- **Local .so File:** `gwimanchor.so`
- **Local File SHA256 Hash:**  
  `7f78f0dabfb48bd6b85c4017fd7ea1686da553fc7381843cb26827b8eb42c263`

## On-Chain Program Information

- **Downloaded Program File:** `program_from_chain.so`
- **On-Chain File SHA256 Hash:**  
  `7f78f0dabfb48bd6b85c4017fd7ea1686da553fc7381843cb26827b8eb42c263`

## Verification Result

The local program binary `gwimanchor.so` and the deployed program binary `program_from_chain.so` have identical SHA256 hashes.

**Therefore, the program deployed on Solana Mainnet has been successfully verified to match the provided source code.**

✅ **Program Verification: SUCCESSFUL**

---

# Date of Verification

- **Date:** April 28, 2025

# Verified by

- **Name:** (Please insert your name or organization name here)

---

# Notes

This verification process was performed by:
- Building the program source locally using `anchor build`
- Downloading the deployed binary using `solana program dump`
- Comparing the SHA256 checksums of both files

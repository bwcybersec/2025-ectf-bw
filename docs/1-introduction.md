\title{BW's eCTF 2025 Satellite TV Design}
\author{BWCyberSec}

\maketitle

# Introduction
This document describes BW's implementation of the Satellite TV encoder and 
decoder for eCTF 2025. This system aims to build a platform to allow the secure 
transmission of TV frames, without allowing for interference or interception.

# Language Choice
We chose to use Rust for our implementation of the decoder. We find that Rust's 
memory safety guarantees are valuable for maintaining the security of our design 
when parsing untrusted input being sent to the decoder. It is important to note
that Rust is not a panacea, and that care has still been taken to ensure that
our design is secure from other vulnerabilities.

\newpage

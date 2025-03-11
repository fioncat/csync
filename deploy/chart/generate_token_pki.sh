#!/bin/bash

openssl genpkey -algorithm RSA -out pki/token_private.pem -pkeyopt rsa_keygen_bits:2048
openssl rsa -pubout -in pki/token_private.pem -out pki/token_public.pem

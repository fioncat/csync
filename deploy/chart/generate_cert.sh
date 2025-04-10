#!/bin/bash

openssl genrsa -out pki/key.pem 2048
openssl req -new -key pki/key.pem -out pki/csr.pem
openssl x509 -req -days 365 -in pki/csr.pem -signkey pki/key.pem -out pki/cert.pem

mkdir /tmp/build
cd /tmp/build
curl https://www.openssl.org/source/openssl-1.1.0h.tar.gz -o openssl.tar.gz
tar xzf openssl.tar.gz && cd openssl-1.1.0h
./Configure linux-x86_64 -fPIC -g no-shared
make -j$(nproc)
cd -

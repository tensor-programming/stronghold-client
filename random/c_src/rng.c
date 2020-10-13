// include stdlib.h and size_t
#include <stdlib.h>
#include <stdint.h>

// variable dependencies and includes.
#if defined(USE_GETRANDOM)
#include <sys/random.h>
#elif defined(USE_ARC4RANDOM)
// stdlib.h is already included.
#elif defined(USE_SECRANDOM)
#include <Security/SecRandom.h>
#elif defined(USE_CRYPTGENRANDOM)
#include <windows.h>
#include <Bcrypt.h>
#elif defined(USE_DEV_RANDOM)
#include <stdio.h>
#endif

// Get a secure series of random bytes and pass the length into a buffer.returns 0 upon success and 1 upon error.
uint8_t os_random_secrandom(uint8_t *buf, size_t len)
{
#if defined(USE_GETRANDOM)
    return getrandom(buf, len, 0) == -1 ? 1 : 0;
#elif defined(USE_ARC4RANDOM)
    arc4random_buf(buf, len);
    return 0;
#elif defined(USE_SECRANDOM)
    return SecRandomCopyBytes(kSecRandomDefault, len, buf) == errSecSuccess ? 0 : 1;
#elif defined(USE_CRYPTGENRANDOM)
    NTSTATUS ret2 = BCryptGenRandom(NULL, buf, (ULONG)len, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    return BCRYPT_SUCCESS(ret2) ? 0 : 1;
#elif defined(USE_DEV_RANDOM)
    FILE *urandom = fopen("/dev/urandom", "r");
    if (urandom == NULL)
        return 1;

    while (len)
    {
        size_t bytes_read = fread(buf, 1, len, urandom);
        if (bytes_read == 0)
        {
            fclose(urandom);
            return 1;
        }

        buf += bytes_read;
        len -= bytes_read;
    }

    fclose(urandom);
    return 0;
#else
#error "No secure random number generator selected"
#endif
}
{ pkgs, flake, ... }:
# End-to-end OIDC login against a real (mock-connector) provider.
#
# Runs Horae with production OIDC enabled (no DEV_LOGIN) alongside a `dex`
# instance whose single `mockCallback` connector auto-authenticates a fixed
# identity (kilgore@kilgore.trout). This exercises the full authorization-code
# flow — /auth/login -> dex -> /auth/callback -> session — and asserts that a
# deactivated user is denied at sign-in (FR-002).
#
# A second phase covers providers whose ID tokens carry no `email_verified`
# claim (tsidp, for example). Stock Dex always sends the claim, so this test
# uses a fixture patch: for the dedicated client `horae-omit-email-verified`
# only, Dex leaves the claim out while building the token, before signing. It is
# a test fixture, not stock Dex and not acceptance against a real provider.
let
  omitClient = "horae-omit-email-verified";

  # Test-only Dex. `--replace-fail` breaks the build if the seam moves.
  dexOmittingEmailVerified = pkgs.dex-oidc.overrideAttrs (old: {
    postPatch = (old.postPatch or "") + ''
      substituteInPlace server/oauth2.go --replace-fail \
        'tok.EmailVerified = &claims.EmailVerified' \
        'if clientID != "${omitClient}" { tok.EmailVerified = &claims.EmailVerified }'
    '';
  });

  # Run Dex's code + PKCE flow for one client without visiting Horae, verify the
  # ID token's signature, issuer, audience and nonce with PyJWT against Dex's
  # JWKS, and print the claims the test relies on.
  idToken = pkgs.writers.writePython3Bin "dex-id-token"
    {
      libraries = with pkgs.python3Packages; [ pyjwt cryptography ];
      flakeIgnore = [ "E501" ];
    } ''
    import base64
    import hashlib
    import http.cookiejar
    import json
    import secrets
    import sys
    import urllib.parse
    import urllib.request

    import jwt

    DEX = "http://127.0.0.1:5556/dex"
    REDIRECT = "http://127.0.0.1:3000/auth/callback"


    class Returned(Exception):
        def __init__(self, url):
            self.url = url


    class StopAtRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            if newurl.startswith(REDIRECT):
                raise Returned(newurl)
            return super().redirect_request(req, fp, code, msg, headers, newurl)


    client_id, client_secret = sys.argv[1], sys.argv[2]
    meta = json.load(urllib.request.urlopen(DEX + "/.well-known/openid-configuration"))
    verifier = secrets.token_urlsafe(48)
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
    state = secrets.token_urlsafe(16)
    nonce = secrets.token_urlsafe(16)
    query = urllib.parse.urlencode({
        "client_id": client_id,
        "redirect_uri": REDIRECT,
        "response_type": "code",
        "scope": "openid email profile",
        "state": state,
        "nonce": nonce,
        "code_challenge": challenge,
        "code_challenge_method": "S256",
    })
    opener = urllib.request.build_opener(
        StopAtRedirect(), urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar())
    )
    try:
        opener.open(meta["authorization_endpoint"] + "?" + query)
        sys.exit("Dex did not redirect back to the client")
    except Returned as returned:
        params = urllib.parse.parse_qs(urllib.parse.urlsplit(returned.url).query)
    assert params["state"] == [state], "state mismatch"

    body = urllib.parse.urlencode({
        "grant_type": "authorization_code",
        "code": params["code"][0],
        "redirect_uri": REDIRECT,
        "code_verifier": verifier,
    }).encode()
    request = urllib.request.Request(meta["token_endpoint"], data=body)
    basic = base64.b64encode(f"{client_id}:{client_secret}".encode()).decode()
    request.add_header("Authorization", "Basic " + basic)
    token = json.load(urllib.request.urlopen(request))["id_token"]

    key = jwt.PyJWKClient(meta["jwks_uri"]).get_signing_key_from_jwt(token)
    claims = jwt.decode(
        token,
        key.key,
        algorithms=["RS256"],
        audience=client_id,
        issuer=meta["issuer"],
        options={"require": ["exp", "iat", "iss", "sub", "aud"]},
    )
    assert claims["nonce"] == nonce, "nonce mismatch"
    print(json.dumps({
        "iss": claims["iss"],
        "sub": claims["sub"],
        "email": claims.get("email"),
        "email_verified_present": "email_verified" in claims,
        "email_verified": claims.get("email_verified"),
    }))
  '';
in
pkgs.testers.nixosTest {
  name = "horae-e2e-oidc";
  nodes.server = { config, lib, ... }: {
    imports = [ flake.nixosModules.horae ];
    systemd.package = import ../test-systemd.nix { inherit pkgs; };
    # Keep guest executable reads off 9p when the builder falls back to TCG.
    virtualisation.useNixStoreImage = true;
    # Preserve the writable overlay needed by register-nix-paths.service.
    virtualisation.writableStore = true;
    # Under TCG, coldplug can exceed the test harness's five-minute device limit.
    # Keep the console dependency alive until the driver's readiness deadline.
    systemd.settings.Manager.DefaultDeviceTimeoutSec = lib.mkForce 900;
    # initdb can exceed PostgreSQL's 120-second startup limit under ARM TCG.
    # Leave the shutdown timeout unchanged.
    systemd.services.postgresql.serviceConfig.TimeoutStartSec = 900;
    # Keep startup diagnostics available before the test driver's console connects.
    systemd.services.test-console-diagnostics = {
      wantedBy = [ "multi-user.target" ];
      path = [ pkgs.systemd pkgs.coreutils ];
      script = ''
        while ! systemctl is-active --quiet backdoor.service; do
          systemctl list-jobs --no-pager || true
          systemctl show backdoor.service dev-hvc0.device dev-ttyAMA0.device dev-ttyS0.device \
            --property=Id,ActiveState,SubState,Result,After,Requires || true
          for device in /dev/hvc0 /dev/ttyAMA0 /dev/ttyS0; do
            udevadm info --query=property --name="$device" || true
          done
          sleep 60
        done
      '';
    };

    services.horae.enable = true;
    services.horae.database.createLocally = true;

    # Production auth: point Horae at the local dex issuer. DEV_LOGIN is unset,
    # so /auth/login starts the OIDC flow and the dev bypass is not registered.
    systemd.services.horae.environment = {
      HORAE_OIDC_ISSUER = "http://127.0.0.1:5556/dex";
      HORAE_OIDC_CLIENT_ID = "horae";
      HORAE_OIDC_CLIENT_SECRET = "horae-e2e-secret";
      HORAE_OIDC_REDIRECT_URL = "http://127.0.0.1:3000/auth/callback";
    };

    # Mock OIDC provider. The mockCallback connector auto-authenticates a fixed
    # verified identity; skipApprovalScreen keeps the flow headless.
    services.dex = {
      enable = true;
      package = dexOmittingEmailVerified;
      settings = {
        issuer = "http://127.0.0.1:5556/dex";
        storage.type = "memory";
        web.http = "127.0.0.1:5556";
        oauth2.skipApprovalScreen = true;
        staticClients = [{
          id = "horae";
          name = "Horae";
          secret = "horae-e2e-secret";
          redirectURIs = [ "http://127.0.0.1:3000/auth/callback" ];
        }
          {
            id = omitClient;
            name = "Horae without email_verified";
            secret = "horae-omit-e2e-secret";
            redirectURIs = [ "http://127.0.0.1:3000/auth/callback" ];
          }];
        connectors = [{
          type = "mockCallback";
          id = "mock";
          name = "Mock";
        }];
      };
    };

    # `horae` to seed/create/link users, `psql` to flip the active flag and
    # inspect bindings, `dex-id-token` to obtain and verify ID tokens.
    environment.systemPackages = [ config.services.horae.package pkgs.postgresql idToken ];
  };

  testScript = ''
    server.start()
    # TCG boots on ARM can exceed the driver's fixed five-minute shell timeout.
    # Wait for the guest's serial readiness signal before connecting to its shell.
    # Read the complete log: wait_for_console_text drains only one line per second.
    retry(lambda _: "connecting to host..." in server.get_console_log(), timeout_seconds=900)
    server.wait_for_unit("register-nix-paths.service")
    server.wait_for_unit("postgresql.service")
    server.wait_for_unit("dex.service")
    server.wait_for_unit("horae.service")
    server.wait_for_open_port(5556)
    server.wait_for_open_port(3000)

    # Health + provider discovery reachable
    server.succeed("curl -sf http://127.0.0.1:3000/health | grep -q ok")
    server.succeed("curl -sf http://127.0.0.1:5556/dex/.well-known/openid-configuration >/dev/null")

    # Seed the org, then create a member whose email matches the mock identity.
    server.succeed("sudo -u horae DATABASE_URL=postgres:///horae horae seed")
    server.succeed(
      "sudo -u horae DATABASE_URL=postgres:///horae horae user create "
      "--email kilgore@kilgore.trout --name 'Kilgore Trout' --role member"
    )

    # /auth/login now serves the SSO landing page; /auth/oidc/start begins the
    # flow. Drive it with a cookie jar, following every redirect:
    # /auth/oidc/start -> dex (mock auto-auth) -> /auth/callback -> /.
    server.succeed(
      "curl -s -c /tmp/jar.txt -b /tmp/jar.txt -L "
      "http://127.0.0.1:3000/auth/oidc/start -o /dev/null"
    )

    # Session established: an authenticated request now succeeds.
    result = server.succeed(
      "curl -s -b /tmp/jar.txt http://127.0.0.1:3000/harvest/v2/time_entries"
    )
    assert '"time_entries"' in result, f"expected authenticated Harvest response, got: {result[:200]}"

    import json
    import shlex

    def horae(args, issuer_env="http://127.0.0.1:5556/dex"):
        return server.execute(
            "sudo -u horae env DATABASE_URL=postgres:///horae "
            f"HORAE_OIDC_ISSUER={shlex.quote(issuer_env)} horae {args} 2>&1"
        )

    def sql(query):
        return server.succeed(f"sudo -u horae psql horae -tAc {shlex.quote(query)}").strip()

    def token(client, secret):
        return json.loads(server.succeed(f"dex-id-token {client} {secret}"))

    def link(user_id, identity):
        return horae(
            f"user link-oidc --user-id {user_id} "
            f"--issuer {shlex.quote(identity['iss'])} --subject {shlex.quote(identity['sub'])}"
        )

    def api_status_after_sign_in(jar):
        # The flow itself must complete: a transport error or a failed
        # redirect chain is a test failure, not a denial.
        server.succeed(
            f"curl -sS -c {jar} -b {jar} -L --max-redirs 8 "
            "http://127.0.0.1:3000/auth/oidc/start -o /dev/null"
        )
        return server.succeed(
            f"curl -sS -o /dev/null -w '%{{http_code}}' -b {jar} "
            "http://127.0.0.1:3000/harvest/v2/time_entries"
        ).strip()

    kilgore = sql("SELECT id FROM users WHERE email = 'kilgore@kilgore.trout'")

    # The normal client's signed ID token carries email_verified = true; it is
    # also the operator's validated evidence of the issuer and subject.
    verified = token("horae", "horae-e2e-secret")
    assert verified["email_verified_present"] is True, verified
    assert verified["email_verified"] is True, verified
    assert verified["email"] == "kilgore@kilgore.trout", verified
    assert verified["iss"] == "http://127.0.0.1:5556/dex", verified

    # Sign-in stored exactly this issuer and subject, so the explicit link is a
    # no-op. Run it before deactivation: an inactive account is refused first.
    status, output = link(kilgore, verified)
    assert status == 0 and "nothing changed" in output, (status, output)

    # FR-002: deactivate the user; a fresh OIDC login must NOT establish a session.
    server.succeed(
      "sudo -u horae psql horae -c "
      "\"UPDATE users SET active = false WHERE email = 'kilgore@kilgore.trout'\""
    )
    # On denial the callback redirects to the /auth/login landing page (a 200,
    # not another flow), so following redirects terminates there. Cap redirects
    # defensively, then confirm no authenticated session was ever established.
    server.execute(
      "curl -s -c /tmp/jar2.txt -b /tmp/jar2.txt -L --max-redirs 8 "
      "http://127.0.0.1:3000/auth/oidc/start -o /dev/null"
    )
    code = server.succeed(
      "curl -s -o /dev/null -w '%{http_code}' -b /tmp/jar2.txt "
      "http://127.0.0.1:3000/harvest/v2/time_entries"
    ).strip()
    assert code != "200", f"deactivated user must not hold an authenticated session, got {code}"

    # Second phase: a provider that never sends email_verified. Reset only this
    # VM's fixture user and point Horae at the omitting client.
    sql(
        "UPDATE users SET active = true, oidc_issuer = NULL, oidc_subject = NULL "
        "WHERE email = 'kilgore@kilgore.trout'"
    )
    server.succeed(
        "mkdir -p /run/systemd/system/horae.service.d && printf '%s\\n' '[Service]' "
        "'Environment=HORAE_OIDC_CLIENT_ID=${omitClient}' "
        "'Environment=HORAE_OIDC_CLIENT_SECRET=horae-omit-e2e-secret' "
        "> /run/systemd/system/horae.service.d/omit-email-verified.conf"
    )
    server.succeed("systemctl daemon-reload && systemctl restart horae.service")
    server.wait_for_open_port(3000)

    # The omitting client's token is signed by the same Dex for the same
    # person, but the optional claim is absent rather than false.
    omitted = token("${omitClient}", "horae-omit-e2e-secret")
    assert omitted["email_verified_present"] is False, omitted
    # The email is still there, so a denial below is about verification only.
    assert omitted["email"] == "kilgore@kilgore.trout", omitted
    assert (omitted["iss"], omitted["sub"]) == (verified["iss"], verified["sub"]), omitted

    # Without a binding the verified-email path cannot apply: denied, unbound.
    status = api_status_after_sign_in("/tmp/jar3.txt")
    assert status == "401", f"absent email_verified must not sign in, got {status}"
    assert sql(
        "SELECT oidc_issuer IS NULL AND oidc_subject IS NULL "
        "FROM users WHERE email = 'kilgore@kilgore.trout'"
    ) == "t"

    # The operator links the identity taken from the validated token.
    status, output = link(kilgore, omitted)
    assert status == 0 and output.startswith(f"Linked user {kilgore}"), (status, output)
    assert omitted["sub"] not in output and omitted["iss"] not in output, output
    status, output = link(kilgore, omitted)
    assert status == 0 and "nothing changed" in output, (status, output)
    assert omitted["sub"] not in output and omitted["iss"] not in output, output

    # The same subject cannot be linked to a second account.
    status, output = horae(
        "user create --email other@kilgore.trout --name Other --role member"
    )
    assert status == 0, output
    other = sql("SELECT id FROM users WHERE email = 'other@kilgore.trout'")
    status, output = link(other, omitted)
    assert status == 1 and "Another user is already linked" in output, (status, output)
    assert omitted["sub"] not in output and omitted["iss"] not in output, output

    # Now the provider's normal flow signs the person in.
    status = api_status_after_sign_in("/tmp/jar4.txt")
    assert status == "200", f"linked identity must sign in without email_verified, got {status}"
  '';
}

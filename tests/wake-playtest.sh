#!/bin/bash
# Runs the wake playtests T-P-4, T-W-1 and T-W-2 against the installed app.
#
#   tests/wake-playtest.sh <ssh alias of the Ollama machine> <chat model>
#
# The model as /v1/models names it, `ollama/<name>`. Not read from there:
# while Ollama sleeps that list holds a placeholder.
#
# Run on the server as an account with passwordless sudo. The Ollama machine
# must run froaky-ai-server, whose `events` show its resumes. Lower its idle
# time first (`froaky settings --idle 2m`) or each wait takes 30 min. The
# script unloads Ollama's models itself: froaky never suspends with one loaded.
# T-W-2 swaps the MAC for a made-up one and puts it back on any exit. The
# page's calls are made on the daemon's loopback port, as nginx would.
set -u

desk=${1:?usage: $0 <ssh alias of the Ollama machine> <chat model>}
model=${2:?usage: $0 <ssh alias of the Ollama machine> <chat model>}
app=whoopy
domain=$(sudo -n yunohost app setting $app domain)
install_dir=$(sudo -n yunohost app setting $app install_dir)
ollama=$(sudo -n yunohost app setting $app ollama_url)
key=$(sudo -n sed -n 's/^LITELLM_MASTER_KEY=//p' "$install_dir/whoopy.env")
web=http://127.0.0.1:$(sudo -n yunohost app setting $app port_web)
fails=0

check() { # check <test> <condition result 0/1> <what>
    if [ "$2" = 0 ]; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; fails=$((fails + 1)); fi
}
awake() { curl -s -m 3 -o /dev/null "$ollama/api/version"; }
status_line() {
    sudo -n "$install_dir/bin/whoopy" --socket "/run/$app/wake.sock" status | head -2 | paste -sd ' '
}
set_settings() { # set_settings <JSON>; the page's Save
    curl -s -m 10 -o /dev/null -w '%{http_code}' -X POST "$web/whoopy/settings" \
        -H 'X-Whoopy: 1' -H 'Content-Type: application/json' -d "$1" | grep -q 200
}
wait_asleep() { # wait_asleep <max s>; prints the seconds waited
    local t0=$SECONDS
    while awake; do
        [ $((SECONDS - t0)) -ge "$1" ] && { echo "still awake after $1 s"; return 1; }
        sleep 10
    done
    echo "asleep after $((SECONDS - t0)) s"
}
unload() {
    curl -s -m 5 "$ollama/api/ps" | jq -r '.models[].name' | while read -r m; do
        curl -s -m 30 -o /dev/null "$ollama/api/generate" -d "{\"model\":\"$m\",\"keep_alive\":0}"
    done
}
chat() { # chat <model>; prints "<http code> <seconds> <body>"
    local t0=$SECONDS body code
    body=$(curl -s -m 600 -w '\n%{http_code}' "https://$domain/v1/chat/completions" \
        -H "Authorization: Bearer $key" -H 'Content-Type: application/json' \
        -d "{\"model\":\"$1\",\"max_tokens\":5,\"messages\":[{\"role\":\"user\",\"content\":\"Say hi.\"}]}")
    code=${body##*$'\n'}
    echo "$code $((SECONDS - t0)) ${body%$'\n'*}"
}
resumes_since() { ssh -o BatchMode=yes "$desk" froaky events | awk -v t="$1" '$1 >= t && /resumed from sleep/' | wc -l; }

echo "model $model, Ollama at $ollama"

echo "-- asleep first"
unload
wait_asleep 600 || exit 1

echo "-- T-P-4: the status line and the page's Wake button"
s=$(status_line); [[ $s == *asleep* ]]; check T-P-4 $? "status asleep: $s"
t0=$SECONDS
out=$(curl -s -m 600 -X POST "$web/whoopy/wake" -H 'X-Whoopy: 1')
[ "$(jq -r .ok <<< "$out")" = true ]
check T-P-4 $? "button in $((SECONDS - t0)) s: $out"
s=$(status_line); [[ $s == *awake*"Last wake: answered after"* ]]; check T-P-4 $? "status awake: $s"
unload
w=$(wait_asleep 600); check T-P-4 $? "$w"

echo "-- T-W-1: a request wakes it, and it suspends again"
mark=$(($(date +%s) - 5))
read -r code secs body < <(chat "$model")
[ "$code" = 200 ]; check T-W-1 $? "chat $code in $secs s"
n=$(resumes_since "$mark"); [ "$n" = 1 ]; check T-W-1 $? "$n resume(s) on the desktop"
ka=$(curl -s -m 5 "$web/whoopy/settings" | jq -r .keep_alive)
if [ -n "$ka" ]; then
    case ${ka: -1} in s) unit=1 ;; m) unit=60 ;; h) unit=3600 ;; esac
    want=$((${ka%?} * unit))
    left=$(( $(date -d "$(curl -s -m 5 "$ollama/api/ps" | jq -r '.models[0].expires_at')" +%s) - $(date +%s) ))
    [ $((want - left)) -ge 0 ] && [ $((want - left)) -lt 120 ]
    check T-W-1 $? "keep_alive $ka: the model unloads in $left s"
fi
s=$(status_line); check T-W-1 0 "status: $s"
unload
w=$(wait_asleep 600); check T-W-1 $? "$w"

echo "-- T-W-2: a wake with no answer fails with a 503 after the timeout"
saved=$(curl -s -m 5 "$web/whoopy/settings")
restore() { set_settings "$saved" && echo "restored the wake settings: $saved"; }
trap restore EXIT
set_settings "$(jq -c '.wol_mac = "02:00:00:00:00:01" | .wake_timeout_s = 20' <<< "$saved")" \
    || { check T-W-2 1 "the page refused the made-up MAC"; exit 1; }
read -r code secs body < <(chat "$model")
[ "$code" = 503 ] && [[ $body == *"did not answer within 20 s"* ]]
check T-W-2 $? "first: $code in $secs s: $(jq -r '.error.message // .' <<< "$body" 2>/dev/null | head -c 200)"
read -r code secs body < <(chat "$model")
[ "$code" = 503 ] && [ "$secs" -lt 3 ]
check T-W-2 $? "second: $code in $secs s: $(jq -r '.error.message // .' <<< "$body" 2>/dev/null | head -c 200)"

echo "-- $fails failure(s)"
[ "$fails" = 0 ]

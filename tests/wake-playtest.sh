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
# T-W-2 swaps the MAC for a made-up one and puts it back on any exit.
set -u

desk=${1:?usage: $0 <ssh alias of the Ollama machine> <chat model>}
model=${2:?usage: $0 <ssh alias of the Ollama machine> <chat model>}
app=whoopy
domain=$(sudo -n yunohost app setting $app domain)
install_dir=$(sudo -n yunohost app setting $app install_dir)
ollama=$(sudo -n yunohost app setting $app ollama_url)
key=$(sudo -n sed -n 's/^LITELLM_MASTER_KEY=//p' "$install_dir/whoopy.env")
fails=0

check() { # check <test> <condition result 0/1> <what>
    if [ "$2" = 0 ]; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; fails=$((fails + 1)); fi
}
awake() { curl -s -m 3 -o /dev/null "$ollama/api/version"; }
status_line() {
    sudo -n yunohost app config get $app main.wake --full | sed -n 's/^ *ask: - //p; s/^- //p' | head -2 | paste -sd ' '
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
gateway_up() { # waits up to 120 s for /v1/models
    for _ in $(seq 24); do
        curl -s -m 5 -o /dev/null -w '%{http_code}' "https://$domain/v1/models" \
            -H "Authorization: Bearer $key" | grep -q 200 && return 0
        sleep 5
    done
    return 1
}
resumes_since() { ssh -o BatchMode=yes "$desk" froaky events | awk -v t="$1" '$1 >= t && /resumed from sleep/' | wc -l; }

echo "model $model, Ollama at $ollama"

echo "-- asleep first"
unload
wait_asleep 600 || exit 1

echo "-- T-P-4: the panel's status line and Wake button"
s=$(status_line); [[ $s == *asleep* ]]; check T-P-4 $? "status asleep: $s"
t0=$SECONDS
out=$(sudo -n yunohost app action run $app main.wake.wake_now 2>&1 | grep -o 'Ollama answers.*')
[[ $out == "Ollama answers, "*" s after the wake request." ]]
check T-P-4 $? "button in $((SECONDS - t0)) s: $out"
s=$(status_line); [[ $s == *awake*"Last wake: answered after"* ]]; check T-P-4 $? "status awake: $s"
unload
w=$(wait_asleep 600); check T-P-4 $? "$w"

echo "-- T-W-1: a request wakes it, and it suspends again"
mark=$(($(date +%s) - 5))
read -r code secs body < <(chat "$model")
[ "$code" = 200 ]; check T-W-1 $? "chat $code in $secs s"
n=$(resumes_since "$mark"); [ "$n" = 1 ]; check T-W-1 $? "$n resume(s) on the desktop"
s=$(status_line); check T-W-1 0 "status: $s"
unload
w=$(wait_asleep 600); check T-W-1 $? "$w"

echo "-- T-W-2: a wake with no answer fails with a 503 after the timeout"
mac=$(sudo -n yunohost app setting $app wol_mac)
timeout_s=$(sudo -n yunohost app setting $app wake_timeout_s)
restore() {
    sudo -n yunohost app config set $app main.wol.wol_mac -v "$mac" >/dev/null 2>&1
    sudo -n yunohost app config set $app main.wol.wake_timeout_s -v "$timeout_s" >/dev/null 2>&1
    gateway_up && echo "restored MAC and timeout ($timeout_s s)"
}
trap restore EXIT
sudo -n yunohost app config set $app main.wol.wol_mac -v 02:00:00:00:00:01 >/dev/null 2>&1
sudo -n yunohost app config set $app main.wol.wake_timeout_s -v 20 >/dev/null 2>&1
gateway_up || { check T-W-2 1 "gateway back after the settings change"; exit 1; }
read -r code secs body < <(chat "$model")
[ "$code" = 503 ] && [[ $body == *"did not answer within 20 s"* ]]
check T-W-2 $? "first: $code in $secs s: $(jq -r '.error.message // .' <<< "$body" 2>/dev/null | head -c 200)"
read -r code secs body < <(chat "$model")
[ "$code" = 503 ] && [ "$secs" -lt 3 ]
check T-W-2 $? "second: $code in $secs s: $(jq -r '.error.message // .' <<< "$body" 2>/dev/null | head -c 200)"

echo "-- $fails failure(s)"
[ "$fails" = 0 ]

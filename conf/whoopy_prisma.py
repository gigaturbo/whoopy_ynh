"""Start Prisma's query engine on Whoopy's reserved port, not a random one.

LiteLLM reaches PostgreSQL through Prisma's query engine, a helper it starts
at each start and talks to over HTTP on loopback. Prisma picks a free port
at random (prisma.engine.utils.get_open_port), and the loopback table
(nftables.conf, B-P-6) allows fixed ports only, so the engine is given
WHOOPY_PRISMA_PORT instead.

Loaded by LiteLLM from `litellm_settings.callbacks` ("whoopy_prisma.pin")
while it reads its config, before it starts the engine. At each move of the
LiteLLM pin, check that prisma/engine/_query.py still calls
`utils.get_open_port()`.
"""

import os

import prisma.engine.utils
from litellm.integrations.custom_logger import CustomLogger

PORT = int(os.environ["WHOOPY_PRISMA_PORT"])
prisma.engine.utils.get_open_port = lambda: PORT

# LiteLLM loads a callback as an object; this one does nothing per request.
pin = CustomLogger()

"""Plugin route hooks only; lifecycle and row execution stay in the shared tools."""


def boot_to_main(ctx):
    """Resolve main/title/known intro in one bounded row, without model-paced inputs."""
    def at(gate):
        try:
            ctx.gate(gate)
            return True
        except RuntimeError:
            return False

    if at('main-fast'):
        return {'screen': 'main', 'route': 'already-main'}
    if at('title-fast'):
        ctx.play('title-confirm')
        ctx.gate('main-settled')
        return {'screen': 'main', 'route': 'positive-title-confirm'}
    ctx.log({'recovery': 'known-intro', 'bounded': True})
    ctx.play('attract-cancel')
    if at('main-fast'):
        return {'screen': 'main', 'route': 'intro-to-main'}
    ctx.gate('title-fast')
    ctx.play('title-confirm')
    ctx.gate('main-settled')
    return {'screen': 'main', 'route': 'intro-title-main'}

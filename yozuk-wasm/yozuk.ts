import { decode } from 'base64-arraybuffer';
import { Result } from './output'

export abstract class YozukBase {
    protected abstract exec_impl(command: string, i18n: string): Promise<string>;
    protected abstract push_stream_impl(stream: Uint8Array): Promise<void>;
    protected abstract random_suggests_impl(amount: number): Promise<string>;
    protected abstract i18n(): I18n;

    async exec(command: string, streams: Uint8Array[] = []): Promise<Result> {
        for (const stream of streams) {
            await this.push_stream_impl(stream);
        }
        const result = JSON.parse(await this.exec_impl(command, JSON.stringify(this.i18n())));
        const textDecoder = new TextDecoder('utf-8', { fatal: true });
        if (result.outputs) {
            result.outputs.forEach((output) => {
                output.blocks.forEach((block) => {
                    const { data } = block;
                    if (data) {
                        const decoded = decode(data);
                        try {
                            block.data = textDecoder.decode(decoded);
                        } catch {
                            block.data = decoded;
                        }
                    }
                });
            });
        }
        return result;
    }

    async random_suggests(amount: number = 5): Promise<String[]> {
        return JSON.parse(await this.random_suggests_impl(amount));
    }
}

export type I18n = {
    locale?: string;
    timezone?: string;
};
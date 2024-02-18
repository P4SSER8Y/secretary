<script setup lang="ts">
import { ref } from 'vue';
import { get_api, p5_message } from '../lib/utils';

const api = get_api();
const emit = defineEmits<{ (e: 'close'): void; (e: 'uploaded', id: string): void }>();
const isPublic = ref(true);

function reset_upload() {
    let fileNode = document.getElementById('file') as HTMLInputElement | null;
    if (fileNode) {
        fileNode.value = '';
    }
    isPublic.value = true;
}

async function upload() {
    let fileNode = document.getElementById('file') as HTMLInputElement | null;
    if (fileNode?.files?.length != 1) {
        p5_message('failed');
        reset_upload();
        return;
    }
    let file = fileNode.files[0];
    p5_message('start');
    let result = await api?.post(
        'new',
        {
            file: file,
            public: isPublic.value,
        },
        {
            headers: {
                'Content-Type': 'multipart/form-data',
            },
        }
    );
    if (result?.status == 200) {
        let json: { ok: boolean; id: string } | null = result.data;
        if (json?.ok) {
            emit('uploaded', json.id);
            reset_upload();
            p5_message('clear');
            emit('close');
            return;
        }
    }
    p5_message('failed');
    reset_upload();
}
</script>

<template>
    <div class="modal-box">
        <div class="grid grid-cols-[6em_minmax(0,1fr)] gap-5">
            <p5-title content="FILE"></p5-title>
            <input type="file" class="file-input file-input-ghost file-input-xs w-full max-w-xs" id="file" />
            <p5-title content="PUBLIC"></p5-title>
            <p5-switch v-model="isPublic"></p5-switch>
            <button @click="upload" class="col-span-2 justify-self-center">
                <p5-title :animation="true" content="SHOOT" size="extra-large"></p5-title>
            </button>
        </div>
    </div>
    <form method="dialog" class="modal-backdrop">
        <button>close</button>
    </form>
</template>

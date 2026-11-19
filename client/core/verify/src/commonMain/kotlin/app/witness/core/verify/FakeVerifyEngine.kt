package app.witness.core.verify

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow

class FakeVerifyEngine(
    private val fallback: Judgement = Judgement.NO_RECORD,
    known: Map<ByteArray, Judgement> = emptyMap(),
) : VerifyEngine {
    private class Key(val bytes: ByteArray) {
        private val hash = bytes.contentHashCode()

        override fun equals(other: Any?): Boolean = other is Key && hash == other.hash && bytes.contentEquals(other.bytes)

        override fun hashCode(): Int = hash
    }

    private val table: Map<Key, Judgement> = known.entries.associate { (k, v) -> Key(k.copyOf()) to v }

    override fun check(input: ByteArray): Flow<CheckUpdate> = flow {
        emit(CheckUpdate.Progress(CheckStage.READ_FILE, StageState.RUNNING))
        if (input.isEmpty()) {
            emit(CheckUpdate.Progress(CheckStage.READ_FILE, StageState.DONE))
            emit(CheckUpdate.Done(Judgement.NO_RECORD))
            return@flow
        }
        val judgement = table[Key(input)] ?: fallback
        emit(CheckUpdate.Progress(CheckStage.READ_FILE, StageState.DONE))
        for (stage in STAGES_AFTER_READ) emit(CheckUpdate.Progress(stage, StageState.DONE))
        emit(CheckUpdate.Done(judgement))
    }

    private companion object {
        val STAGES_AFTER_READ = CheckStage.entries.drop(1)
    }
}

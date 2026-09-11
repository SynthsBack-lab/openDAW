import {describe, expect, it} from "vitest"
import {isDefined, UUID} from "@opendaw/lib-std"
import {AudioData} from "@opendaw/lib-dsp"
import {BpmDetector, Sample} from "@opendaw/studio-adapters"
import {AssetService} from "../AssetService"
import {SampleService} from "./SampleService"

// fixes #375: importRecording derived the AudioFileBox uuid from the WAV bytes (content addressing).
// Two takes with byte-identical audio (two mics on one signal, or silence recorded twice) then
// collided on the same uuid and the second AudioFileBox.create panicked "already staged".

class CapturingSampleService extends SampleService {
    readonly imports: Array<AssetService.ImportArgs> = []

    constructor() {super({} as AudioContext, BpmDetector.Unknown)}

    async importFile(args: AssetService.ImportArgs): Promise<Sample> {
        this.imports.push(args)
        const uuid = args.uuid ?? UUID.generate()
        return {uuid: UUID.toString(uuid), name: "", bpm: 0, duration: 1, sample_rate: 48000, origin: "recording"}
    }
}

const createSilence = () => AudioData.create(48000, 4800, 1)

describe("SampleService.importRecording", () => {
    it("assigns every take its own uuid, even for byte-identical audio", async () => {
        const service = new CapturingSampleService()
        const first = await service.importRecording(createSilence(), 120)
        const second = await service.importRecording(createSilence(), 120)
        expect(service.imports).toHaveLength(2)
        service.imports.forEach(args => expect(isDefined(args.uuid)).toBe(true))
        expect(first.uuid).not.toBe(second.uuid)
    })
})

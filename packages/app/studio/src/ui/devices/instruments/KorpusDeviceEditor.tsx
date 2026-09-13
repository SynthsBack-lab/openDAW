import css from "./KorpusDeviceEditor.sass?inline"
import {Lifecycle} from "@opendaw/lib-std"
import {createElement} from "@opendaw/lib-jsx"
import {DeviceEditor} from "@/ui/devices/DeviceEditor.tsx"
import {MenuItems} from "@/ui/devices/menu-items.ts"
import {ControlBuilder} from "@/ui/devices/ControlBuilder.tsx"
import {DevicePeakMeter} from "@/ui/devices/panel/DevicePeakMeter.tsx"
import {AutomatableParameterFieldAdapter, DeviceHost, KorpusDeviceBoxAdapter, KorpusPresets} from "@opendaw/studio-adapters"
import {Html} from "@opendaw/lib-dom"
import {StudioService} from "@/service/StudioService"
import {Colors, IconSymbol} from "@opendaw/studio-enums"
import {MenuItem} from "@opendaw/studio-core"

const className = Html.adoptStyleSheet(css, "KorpusDeviceEditor")

type Construct = {
    lifecycle: Lifecycle
    service: StudioService
    adapter: KorpusDeviceBoxAdapter
    deviceHost: DeviceHost
}

export const KorpusDeviceEditor = ({lifecycle, service, adapter, deviceHost}: Construct) => {
    const {project} = service
    const {editing, midiLearning} = project
    const {
        exciter, intensity, position, vibrato,
        objectA, dampingA, tuneA, widthA,
        objectB, dampingB, tuneB, detuneB, widthB, levelB,
        routing, couple, volume
    } = adapter.namedParameter
    const box = adapter.box
    const knob = (parameter: AutomatableParameterFieldAdapter, label?: string) =>
        ControlBuilder.createKnob({lifecycle, editing, midiLearning, adapter, parameter, color: Colors.black, label})
    const objectBKnobs: HTMLElement = (
        <div className="knobs">
            {knob(objectB, "Object")}
            {knob(dampingB, "Damping")}
            {knob(tuneB, "Tune")}
            {knob(detuneB, "Detune")}
            {knob(widthB, "Width")}
            {knob(levelB, "Level")}
        </div>
    )
    lifecycle.own(objectB.catchupAndSubscribe(owner =>
        objectBKnobs.classList.toggle("bypassed", owner.getValue() === 6)))
    return (
        <DeviceEditor lifecycle={lifecycle}
                      service={service}
                      adapter={adapter}
                      populateMenu={parent => {
                          MenuItems.forAudioUnitInput(parent, service, deviceHost)
                          parent.addMenuItem(MenuItem.default({label: "Presets", separatorBefore: true})
                              .setRuntimeChildrenProcedure(submenu => submenu.addMenuItem(...KorpusPresets.Factory
                                  .map(preset => MenuItem.default({label: preset.name})
                                      .setTriggerProcedure(() => editing.modify(() =>
                                          KorpusPresets.apply(box, preset)))))))
                      }}
                      populateControls={() => (
                          <div className={className}>
                              <section className="zone">
                                  <h5>Exciter</h5>
                                  <div className="quad">
                                      {knob(exciter)}
                                      {knob(intensity)}
                                      {knob(position)}
                                      {knob(vibrato)}
                                  </div>
                              </section>
                              <section className="zone">
                                  <h5>Objects</h5>
                                  <div className="rows">
                                      <div className="row">
                                          <span className="tag">A</span>
                                          <div className="knobs">
                                              {knob(objectA, "Object")}
                                              {knob(dampingA, "Damping")}
                                              {knob(tuneA, "Tune")}
                                              {knob(widthA, "Width")}
                                          </div>
                                      </div>
                                      <div className="row">
                                          <span className="tag">B</span>
                                          {objectBKnobs}
                                      </div>
                                  </div>
                              </section>
                              <section className="zone last">
                                  <h5>Out</h5>
                                  <div className="quad">
                                      {knob(routing)}
                                      {knob(couple)}
                                      {knob(volume)}
                                  </div>
                              </section>
                          </div>
                      )}
                      populateMeter={() => (
                          <DevicePeakMeter lifecycle={lifecycle}
                                           receiver={project.liveStreamReceiver}
                                           address={adapter.address}/>
                      )}
                      icon={IconSymbol.DrumSet}/>
    )
}
